mod ses_email_client;

use crate::domain::SubscriberEmail;
use async_trait::async_trait;
pub use ses_email_client::SesEmailClient;

#[async_trait]
pub trait Email: Send + Sync {
    async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), anyhow::Error>;
}
