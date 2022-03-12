use std::panic;
use std::fs::File;
use std::io::Read;
use std::process::exit;
use lettre::smtp::authentication::Credentials;
use lettre::{ClientSecurity, Transport};
use lettre_email::{Email, EmailBuilder};
use log::error;
use serde::Deserialize;
use tide::security::Origin;

const NAME_CN: &str = "遇见雨燕";
const NAME_EN: &str = "Swift's Moment";

const EMAIL_CN: &str = r#"<meta charset="utf8">
<div class="container">
	<h1>%name%</h1>
	<div class="content">
		<p>致 <span class="username">%username%</span>:</p>
		<p>您的邮件验证码是:</p>
		<div class="code">
			%code%
		</div>
		<p>请打开 <a  href="%url%">%url%</a> 应用该验证码！</p>
		<p>此验证码有效期为 <span class="expire">%expire%</span> 分钟，请尽快完成相应操作！</p>
		<center class="tip">如果您没有进行任何操作，请忽略该邮件</center>
	</div>
</div>
<style>
	.container {
		text-align: center;
		max-width: 500px;
		width: 100%;
		padding: 5px 30px 30px;

	}
	.content {
		width: 100%;
		text-align: left;
	}
	.username {
		font-weight: 100;
		color: #5D5D5D;
	}
	.code {

		border-radius: 10px;
		width: 100%;
		height:  70px;
		border: #5D5D5D solid 1px;
		text-align: center;
		line-height: 70px;
		font-size: 30px;
		font-weight: 300;
		letter-spacing: 5px;
	}
	.expire {
		font-weight: 500;
		color: #18a058;
	}
	a {
		color: #18a058;
		text-decoration: none;
	}
	.tip {
		margin-top: 30px;
		color: #5D5D5D;
		font-size: 13px;
	}
</style>"#;

const EMAIL_EN: &str = r#"<div class="container">
	<h1>%name%</h1>
	<div class="content">
		<p>To <span class="username">%username%</span>:</p>
		<p>Your email verification code is:</p>
		<div class="code">
			%code%
		</div>
		<p>Please open <a  href="%url%">%url%</a> to apply this verification code!</p>
		<p>This verification code is valid for <span class="expire">%expire%</span> minutes, please complete it as soon as possible!</p>
		<center class="tip">If you don't take any action, please ignore this message</center>
	</div>
</div>
<style>
	.container {
		text-align: center;
		max-width: 500px;
		width: 100%;
		padding: 5px 30px 30px;

	}
	.content {
		width: 100%;
		text-align: left;
	}
	.username {
		font-weight: 100;
		color: #5D5D5D;
	}
	.code {

		border-radius: 10px;
		width: 100%;
		height:  70px;
		border: #5D5D5D solid 1px;
		text-align: center;
		line-height: 70px;
		font-size: 30px;
		font-weight: 300;
		letter-spacing: 5px;
	}
	.expire {
		font-weight: 500;
		color: #18a058;
	}
	.tip {
		margin-top: 30px;
		color: #5D5D5D;
		font-size: 13px;
	}
	a {
		color: #18a058;
		text-decoration: none;
	}
</style>"#;


#[derive(Deserialize, Debug, Clone)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct StorageConfig {
    pub path: String,
}

impl StorageConfig {
    pub fn get_path(&self, filename: String) -> String {
        format!("{}/{}", self.path, filename)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SessionConfig {
    pub logout_on_ip_change: bool,
    pub timeout: u64,
}


#[derive(Deserialize, Debug, Clone)]
pub struct EmailConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}


// 懒得新弄一个模块了，邮件相关逻辑放在这里
impl EmailConfig {
    pub fn send(&self, mail: Email) -> Result<(), ()> {
        let cred = Credentials::new(self.username.clone(), self.password.clone());
        let mailer = lettre::SmtpClient::new(format!("{}:{}", self.host, self.port), ClientSecurity::None);
        if mailer.is_err() {
            return Err(());
        }
        let mut mailer = mailer.unwrap()
            .credentials(cred)
            .transport();
        if let Ok(..) = mailer.send(mail.into()) {
            Ok(())
        } else {
            Err(())
        }
    }
    // 制作一个验证码邮件
    pub fn code_letter(&self, config: &ServerConfig, code: String, name: String, lang: String, to_email: String) -> Email {
        if lang == "cn" {
            let body = EMAIL_CN.replace("%url%", &format!("{}/code/{}", config.base_url, code))
                .replace("%code%", &code.to_string())
                .replace("%name%", NAME_CN)
                .replace("%username%", &name);

            EmailBuilder::new()
                .to((to_email, name))
                .from((self.from.as_str(), format!("[{}]", NAME_CN)))
                .subject(format!("[{}] 验证码", NAME_CN))
                .html(body)
                .build().unwrap()
        } else {
            let body = EMAIL_EN.replace("%url%", &format!("{}/code/{}", config.base_url, code))
                .replace("%code%", &code.to_string())
                .replace("%name%", NAME_EN)
                .replace("%username%", &name);
            EmailBuilder::new()
                .to((to_email, name))
                .from((self.from.as_str(), format!("[{}]", NAME_EN)))
                .subject(format!("[{}] Verification Code", NAME_EN))
                .html(body)
                .build().unwrap()
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Model {
    pub name: String,
    pub path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AiConfig {
    pub models: Vec<Model>,
}

impl AiConfig {
    pub fn get_model_path(&self, name: &str) -> Option<String> {
        for model in &self.models {
            if model.name == name {
                return Some(model.path.clone());
            }
        }
        None
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub origins: Vec<String>,
    pub base_url: String,
}

impl ServerConfig {
    pub fn get_origins(&self) -> Origin {
        Origin::from(self.origins.clone())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub session: SessionConfig,
    pub email: EmailConfig,
    pub ai: AiConfig,
    pub server: ServerConfig,
}

fn _load_config() -> Config {
    let mut file = File::open("config.toml").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let config: Config = toml::from_str(&contents).unwrap();
    config
}

pub fn load_config() -> Config {
    panic::catch_unwind(|| {
        _load_config()
    }).unwrap_or_else(|_| {
        error!("无法加载配置文件!");
        exit(1);
    })
}