use async_nats::jetstream;
use async_nats::jetstream::consumer::PullConsumer;
use crate::models::BuildJob;

/// NATS JetStream-based build queue.
/// Replaces the SQLite polling loop with push-based, exactly-once delivery.
#[derive(Clone)]
pub struct BuildQueue {
    client: async_nats::Client,
    jetstream: jetstream::Context,
    stream_name: String,
    consumer_name: String,
}

const BUILD_SUBJECT: &str = "builds.new";
const BUILD_RETRY_SUBJECT: &str = "builds.retry";
const BUILD_DLQ_SUBJECT: &str = "builds.dlq";
const MAX_DELIVERY_ATTEMPTS: u32 = 3;

impl BuildQueue {
    /// Connect to NATS and ensure the JetStream stream exists
    pub async fn connect(nats_url: &str, stream_name: &str, consumer_name: &str) -> anyhow::Result<Self> {
        let client = async_nats::connect(nats_url).await?;
        let jetstream = jetstream::new(client.clone());

        // Create or update the stream with work-queue retention
        // Work-queue ensures each message is delivered to exactly one consumer
        let stream = jetstream
            .create_stream(jetstream::stream::Config {
                name: stream_name.to_string(),
                subjects: vec![
                    "builds.>".to_string(),
                ],
                retention: jetstream::stream::RetentionPolicy::WorkQueue,
                max_messages: 10_000,
                max_bytes: 1024 * 1024 * 1024, // 1GB
                storage: jetstream::stream::StorageType::File,
                num_replicas: 1,
                discard: jetstream::stream::DiscardPolicy::Old,
                ..Default::default()
            })
            .await?;

        tracing::info!(
            "NATS JetStream connected — stream '{}' ready ({} messages pending)",
            stream_name,
            stream.cached_info().state.messages
        );

        Ok(Self {
            client,
            jetstream,
            stream_name: stream_name.to_string(),
            consumer_name: consumer_name.to_string(),
        })
    }

    /// Publish a build job to the queue (called by API when a deployment is created)
    pub async fn enqueue(&self, job: &BuildJob) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(job)?;
        self.jetstream
            .publish(BUILD_SUBJECT, payload.into())
            .await?;

        tracing::info!(
            "Enqueued build job for deployment {} (attempt {})",
            job.deployment_id,
            job.attempt
        );
        Ok(())
    }

    /// Re-enqueue a failed build for retry
    pub async fn retry(&self, job: &BuildJob) -> anyhow::Result<()> {
        if job.attempt >= MAX_DELIVERY_ATTEMPTS {
            return self.dead_letter(job).await;
        }

        let mut retry_job = job.clone();
        retry_job.attempt += 1;

        let payload = serde_json::to_vec(&retry_job)?;
        self.jetstream
            .publish(BUILD_RETRY_SUBJECT, payload.into())
            .await?;

        tracing::info!(
            "Retrying build job {} (attempt {}/{})",
            retry_job.deployment_id,
            retry_job.attempt,
            MAX_DELIVERY_ATTEMPTS
        );
        Ok(())
    }

    /// Send a permanently failed build to the dead-letter queue for inspection/replay
    pub async fn dead_letter(&self, job: &BuildJob) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(job)?;
        self.jetstream
            .publish(BUILD_DLQ_SUBJECT, payload.into())
            .await?;

        tracing::error!(
            "Build job {} sent to dead-letter queue (exhausted {} attempts)",
            job.deployment_id,
            MAX_DELIVERY_ATTEMPTS
        );
        Ok(())
    }

    /// Create a pull consumer for build workers
    /// Workers call this and then consume messages in a loop
    pub async fn create_consumer(&self) -> anyhow::Result<PullConsumer> {
        let consumer = self
            .jetstream
            .create_consumer_on_stream(
                jetstream::consumer::pull::Config {
                    durable_name: Some(self.consumer_name.clone()),
                    ack_policy: jetstream::consumer::AckPolicy::Explicit,
                    max_deliver: MAX_DELIVERY_ATTEMPTS as i64,
                    ack_wait: std::time::Duration::from_secs(600), // 10 min ack timeout
                    filter_subjects: vec![
                        BUILD_SUBJECT.to_string(),
                        BUILD_RETRY_SUBJECT.to_string(),
                    ],
                    ..Default::default()
                },
                self.stream_name.clone(),
            )
            .await?;

        tracing::info!("NATS consumer '{}' created", self.consumer_name);
        Ok(consumer)
    }

    /// Get the number of pending messages in the queue
    pub async fn pending_count(&self) -> anyhow::Result<u64> {
        let info = self.jetstream.get_stream(&self.stream_name).await?;
        Ok(info.cached_info().state.messages)
    }

    /// Subscribe to build status updates (for WebSocket streaming)
    pub async fn subscribe_status(&self) -> anyhow::Result<async_nats::Subscriber> {
        let sub = self.client.subscribe("builds.status.*").await?;
        Ok(sub)
    }

    /// Publish a build status update
    pub async fn publish_status(&self, deployment_id: &str, status: &str, message: &str) -> anyhow::Result<()> {
        let subject = format!("builds.status.{}", deployment_id);
        let payload = serde_json::json!({
            "deployment_id": deployment_id,
            "status": status,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        self.client.publish(subject, serde_json::to_vec(&payload)?.into()).await?;
        Ok(())
    }
}
