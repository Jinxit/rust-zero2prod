use crate::domain::SubscriberEmail;

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
