use anyhow::{Context, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
#[derive(Debug, Serialize, Deserialize)]
pub struct AffiliateInfo {
    pub code: String,
    pub email: Option<String>,
    pub user_id: String,
    pub referral_link: String,
    pub created_at: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AffiliateResponse {
    pub success: bool,
    pub affiliate_code: Option<String>,
    pub referral_link: Option<String>,
    pub message: String,
    pub commission_rate: String,
    pub payout_schedule: String,
}
pub fn generate_affiliate_code() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}
fn get_affiliate_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".shipwreck")
        .join("affiliate")
}
pub fn save_affiliate_info(info: &AffiliateInfo) -> Result<()> {
    let dir = get_affiliate_dir();
    fs::create_dir_all(&dir).context("Failed to create affiliate directory")?;
    let info_path = dir.join("info.json");
    let json = serde_json::to_string_pretty(info)?;
    fs::write(&info_path, json).context("Failed to save affiliate info")?;
    let code_path = dir.join("code");
    fs::write(&code_path, &info.code).context("Failed to save affiliate code")?;
    Ok(())
}
pub fn load_affiliate_info() -> Result<Option<AffiliateInfo>> {
    let info_path = get_affiliate_dir().join("info.json");
    if !info_path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(&info_path).context("Failed to read affiliate info")?;
    let info: AffiliateInfo = serde_json::from_str(&json)?;
    Ok(Some(info))
}
pub async fn register_affiliate(email: Option<String>) -> Result<AffiliateResponse> {
    let code = generate_affiliate_code();
    let user_id = format!(
        "cm_{}_{}", chrono::Utc::now().timestamp(), std::process::id()
    );
    let email = email
        .or_else(|| {
            std::process::Command::new("git")
                .args(&["config", "--global", "user.email"])
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        String::from_utf8(output.stdout)
                            .ok()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                    } else {
                        None
                    }
                })
        });
    let payload = serde_json::json!(
        { "email" : email, "user_id" : user_id, "name" : "Cargo Mate User",
        "affiliate_code" : code }
    );
    let client = reqwest::Client::new();
    let response = client
        .post("https://cargo.do/api/create-affiliate")
        .json(&payload)
        .send()
        .await;
    match response {
        Ok(resp) if resp.status().is_success() => {
            let api_response: AffiliateResponse = resp.json().await?;
            let info = AffiliateInfo {
                code: api_response.affiliate_code.clone().unwrap_or(code.clone()),
                email: email.clone(),
                user_id,
                referral_link: format!(
                    "https://cargo.do/mates/{}/", api_response.affiliate_code.as_ref()
                    .unwrap_or(& code)
                ),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            save_affiliate_info(&info)?;
            Ok(api_response)
        }
        _ => {
            let info = AffiliateInfo {
                code: code.clone(),
                email,
                user_id,
                referral_link: format!("https://cargo.do/mates/{}/", code),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            save_affiliate_info(&info)?;
            Ok(AffiliateResponse {
                success: true,
                affiliate_code: Some(code.clone()),
                referral_link: Some(format!("https://cargo.do/mates/{}/", code)),
                message: "Affiliate code generated locally".to_string(),
                commission_rate: "33%".to_string(),
                payout_schedule: "quarterly".to_string(),
            })
        }
    }
}
pub fn display_affiliate_info() -> Result<()> {
    if let Some(info) = load_affiliate_info()? {
        use colored::*;
        println!(
            "\n{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .cyan()
        );
        println!("  {} {}", "🎉".yellow(), "Your Affiliate Information".bold());
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .cyan()
        );
        println!();
        println!("  {} {}", "Code:".bold(), info.code.magenta().bold());
        println!("  {} {}", "Link:".bold(), info.referral_link.blue());
        if let Some(email) = &info.email {
            println!("  {} {}", "Email:".bold(), email);
        }
        println!();
        println!("  {} Earn 33% commission on all referrals", "•".cyan());
        println!("  {} Quarterly payouts", "•".cyan());
        println!("  {} Lifetime tracking", "•".cyan());
        println!();
        println!("  {} {}", "Learn more:".dimmed(), "https://cargo.do/mates".blue());
        println!("  {} {}", "BBL License:".dimmed(), "https://cargo.do/license".blue());
        println!();
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .cyan()
        );
        println!(
            "  {} {}", "⚓".magenta(), "Happy sailing & happy earning! 🚢 💰".bold()
        );
        println!("  {}", "Rust On!".bold().yellow());
        println!();
    } else {
        println!(
            "No affiliate information found. Run 'cm affiliate register' to join the program."
        );
    }
    Ok(())
}
pub fn show_affiliate_program_info() -> Result<()> {
    use colored::*;
    println!(
        "\n{}",
        "🚢━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━🚢"
        .cyan().bold()
    );
    println!(
        "  {} {} {}", "💰".yellow(), "CARGO MATE AFFILIATE PROGRAM".bold().yellow(),
        "💰".yellow()
    );
    println!(
        "  {} {} {}", "🎯".magenta(), "EARN 33% COMMISSION ON ALL REFERRALS".bold()
        .magenta(), "🎯".magenta()
    );
    println!(
        "{}",
        "🚢━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━🚢"
        .cyan().bold()
    );
    println!();
    if let Some(info) = load_affiliate_info()? {
        println!(
            "  {} {}", "✅".green(), "You already have an affiliate account!".bold()
            .green()
        );
        println!();
        println!("  {} {}", "Your Code:".bold(), info.code.magenta().bold());
        println!("  {} {}", "Share Link:".bold(), info.referral_link.blue().underline());
        println!();
    } else {
        println!(
            "  {} {}", "🎯".yellow(), "Join our affiliate program and start earning!"
            .bold()
        );
        println!();
        println!("  {} Get your unique affiliate link", "•".cyan());
        println!("  {} Share with other Rust developers", "•".cyan());
        println!("  {} Earn 33% on every referral", "•".cyan());
        println!("  {} Quarterly payouts", "•".cyan());
        println!();
        println!(
            "  {} {}", "How to join:".bold(), "cm affiliate register".cyan().bold()
        );
        println!();
    }
    println!("  {} {}", "📊 Commission Rate:".bold(), "33%".green().bold());
    println!("  {} {}", "⏰ Payout Schedule:".bold(), "Quarterly".blue().bold());
    println!("  {} {}", "🔗 Tracking:".bold(), "Lifetime".magenta().bold());
    println!();
    println!(
        "  {} {}", "📈 Learn more:".dimmed(), "https://cargo.do/mates".blue()
        .underline()
    );
    println!(
        "  {} {}", "📜 BBL License:".dimmed(), "https://cargo.do/license".blue()
        .underline()
    );
    println!();
    println!(
        "{}",
        "🚢━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━🚢"
        .cyan().bold()
    );
    println!("  {} {}", "⚓".magenta(), "Share Cargo Mate & Earn Together!".bold());
    println!("  {}", "Rust On! 🦀".yellow().bold());
    println!();
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_generate_affiliate_code() {
        let code = generate_affiliate_code();
        assert_eq!(code.len(), 8);
        assert!(code.chars().all(| c | "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".contains(c)));
    }
}