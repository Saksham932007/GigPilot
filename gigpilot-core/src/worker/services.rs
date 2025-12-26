use tracing::{info, warn};

/// Mock LLM service for generating email content.
/// 
/// In production, this would call an actual LLM API (OpenAI, Anthropic, etc.)
/// to generate personalized email content based on the tone and context.
/// 
/// # Arguments
/// 
/// * `tone` - The tone of the email ("polite" or "firm")
/// * `context` - Context about the invoice (client name, amount, due date, etc.)
/// 
/// # Returns
/// 
/// Returns a mock email subject and body as a tuple.
/// 
/// # Example
/// 
/// ```rust
/// let (subject, body) = generate_email("polite", "Invoice INV-001 for $100.00");
/// ```
pub async fn generate_email(tone: &str, context: &str) -> Result<(String, String), anyhow::Error> {
    info!("Mock LLM: Generating {} email with context: {}", tone, context);
    
    // Simulate async LLM call delay
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    let (subject, body) = match tone {
        "polite" => (
            "Friendly Reminder: Payment Due".to_string(),
            format!(
                "Dear Client,\n\nThis is a friendly reminder regarding {}. \
                We hope this message finds you well.\n\n\
                We wanted to gently remind you that payment is now due. \
                We appreciate your prompt attention to this matter.\n\n\
                Thank you for your business!\n\nBest regards,\nGigPilot",
                context
            ),
        ),
        "firm" => (
            "Urgent: Payment Required".to_string(),
            format!(
                "Dear Client,\n\nThis is an urgent reminder regarding {}. \
                Payment is now overdue and requires immediate attention.\n\n\
                We have previously sent reminders, and we need to receive \
                payment as soon as possible. Please arrange payment \
                immediately to avoid further action.\n\n\
                We look forward to resolving this matter promptly.\n\n\
                Best regards,\nGigPilot",
                context
            ),
        ),
        _ => {
            warn!("Unknown tone: {}, defaulting to polite", tone);
            generate_email("polite", context).await?
        }
    };
    
    info!("Mock LLM: Generated email subject: {}", subject);
    Ok((subject, body))
}

/// Mock email sending service.
/// 
/// In production, this would integrate with an email service provider
/// (SendGrid, AWS SES, Mailgun, etc.) to actually send emails.
/// 
/// # Arguments
/// 
/// * `to` - Recipient email address
/// * `subject` - Email subject line
/// * `body` - Email body content
/// 
/// # Returns
/// 
/// Returns `Ok(())` if the email was sent successfully, or an error.
/// 
/// # Example
/// 
/// ```rust
/// send_email("client@example.com", "Reminder", "Please pay...").await?;
/// ```
pub async fn send_email(to: &str, subject: &str, body: &str) -> Result<(), anyhow::Error> {
    info!("Mock Email Service: Sending email to {}", to);
    info!("Subject: {}", subject);
    info!("Body preview: {}...", &body[..body.len().min(100)]);
    
    // Simulate async email sending delay
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    
    // In production, this would be:
    // let client = EmailClient::new();
    // client.send(Email {
    //     to: to.to_string(),
    //     subject: subject.to_string(),
    //     body: body.to_string(),
    // }).await?;
    
    info!("Mock Email Service: Email sent successfully to {}", to);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_polite_email() {
        let (subject, body) = generate_email("polite", "Invoice INV-001")
            .await
            .expect("Should generate email");
        
        assert!(subject.contains("Friendly"));
        assert!(body.contains("friendly reminder"));
    }

    #[tokio::test]
    async fn test_generate_firm_email() {
        let (subject, body) = generate_email("firm", "Invoice INV-001")
            .await
            .expect("Should generate email");
        
        assert!(subject.contains("Urgent"));
        assert!(body.contains("overdue"));
    }

    #[tokio::test]
    async fn test_send_email() {
        let result = send_email(
            "test@example.com",
            "Test Subject",
            "Test body content",
        )
        .await;
        
        assert!(result.is_ok());
    }
}

