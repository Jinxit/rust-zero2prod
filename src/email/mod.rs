mod ses_email_client;

use crate::domain::SubscriberEmail;
pub use ses_email_client::SesEmailClient;

#[async_trait]
pub trait Email {
    async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> anyhow::Result<()>;
}
