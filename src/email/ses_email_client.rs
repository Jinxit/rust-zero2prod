use crate::domain::SubscriberEmail;
use crate::email::email::Email;
use aws_sdk_sesv2 as ses;
use aws_sdk_sesv2::model::{Body, Content, Destination, EmailContent, Message};

pub struct SesEmailClient {
    ses_client: ses::Client,
    sender: SubscriberEmail,
}

impl SesEmailClient {
    pub fn new(ses_client: ses::Client, sender: SubscriberEmail) -> Self {
        Self { ses_client, sender }
    }
}

#[async_trait]
impl Email for SesEmailClient {
    async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> anyhow::Result<()> {
        let html_content = Content::builder()
            .data(html_content)
            .charset("UTF-8")
            .build();
        let text_content = Content::builder()
            .data(text_content)
            .charset("UTF-8")
            .build();
        let body = Body::builder()
            .html(html_content)
            .text(text_content)
            .build();
        let subject = Content::builder().data(subject).charset("UTF-8").build();
        let message = Message::builder().subject(subject).body(body).build();
        let content = EmailContent::builder().simple(message).build();
        let destination = Destination::builder()
            .to_addresses(recipient.as_ref())
            .build();

        self.ses_client
            .send_email()
            .from_email_address(self.sender.as_ref())
            .destination(destination)
            .content(content)
            .send()
            .await?;
        Ok(())
    }
}
