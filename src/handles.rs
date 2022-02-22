use std::process::Command;
use telegram_bot::prelude::*;
use telegram_bot::*;

fn get_name(url: &str) -> Result<String, std::io::Error> {
    let name = Command::new("youtube-dl")
        .args(["--skip-download", "--get-title", &url])
        .output()?
        .stdout;
    let name = std::str::from_utf8(&name)
        .unwrap_or_else(|_| { "unknown_name" })
        .to_string();
    Ok(name)
}

pub async fn vid_handle(api: Api, message: Message) -> Result<(), Error> {
    if message.text().unwrap().len() < 5 {
        return Ok(());
    }
    let link = &message.text().unwrap()[5..];
    let mut vid_name = get_name(&link).unwrap_or_else(|_| { "unknown_name".to_string() });
    vid_name.push_str(".mp4");
    let vid = Command::new("youtube-dl")
        .args(["-o", "-", &link])
        .output();
    if let Ok(vid) = vid {
        if vid.stdout.is_empty() {
            api.send(message.text_reply("Some error occured")).await?;
        }
        let vid = InputFileUpload::with_data(vid.stdout, vid_name);
        api.send(message.chat.video(vid)).await?;
        api.send(message.delete()).await?;
    } else {
        api.send(message.text_reply("Something went wrong!")).await?;
    }
    Ok(())
}

pub async fn mus_handle(api: Api, message: Message) -> Result<(), Error> {
    if message.text().unwrap().len() < 5 {
        return Ok(());
    }
    let link = &message.text().unwrap()[5..];
    let mut vid_name = get_name(&link).unwrap_or_else(|_| { "unknown_name".to_string() });
    vid_name.push_str(".mp3");
    let aud = Command::new("youtube-dl")
        .args(["-f", "bestaudio[ext=m4a]", "-o", "-", &link])
        .output();
    if let Ok(aud) = aud {
        if aud.stdout.is_empty() {
            api.send(message.text_reply("Some error occured")).await?;
        }
        let aud = InputFileUpload::with_data(aud.stdout, vid_name);
        api.send(message.chat.audio(aud)).await?;
        api.send(message.delete()).await?;
    } else {
        api.send(message.text_reply("Something went wrong!")).await?;
    }
    Ok(())
}
