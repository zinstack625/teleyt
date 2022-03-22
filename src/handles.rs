use std::process::Command;
use telegram_bot::prelude::*;
use telegram_bot::*;
use tempfile::tempdir;

fn get_name(url: &str) -> Result<String, std::io::Error> {
    let name = Command::new("yt-dlp")
        .args(["--skip-download", "--get-title", &url])
        .output()?
        .stdout;
    let name = std::str::from_utf8(&name)
        .unwrap_or_else(|_| { "unknown_name" })
        .to_string();
    Ok(name)
}

fn dwnld_file(link: &str, name: &str) -> std::io::Result<(String, tempfile::TempDir)> {
    let dir = tempdir()?;
    let filepath = dir.path().join(name);
    let filename = filepath.to_str();
    if filename.is_none() {
        return Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Unable to get filepath for youtube-dl"));
    }
    let filename = filename.unwrap().to_string();
    Command::new("yt-dlp")
        .args(["-f", "[filesize_approx<=50m]", "-o", &filename, "-v", &link])
        .status()?;
    Ok((filename, dir))
}

pub async fn vid_handle_in_mem(api: Api, message: Message) -> Result<(), Error> {
    if message.text().unwrap().len() < 5 {
        return Ok(());
    }
    let link = &message.text().unwrap()[5..];
    let mut vid_name = get_name(&link).unwrap_or_else(|_| { "unknown_name".to_string() });
    vid_name.push_str(".mp4");
    let vid = Command::new("yt-dlp")
        .args(["-f", "[filesize_approx<=50m]", "-o", "-", &link])
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

pub async fn vid_handle(api: Api, message: Message) -> Result<(), Error> {
    if message.text().unwrap().len() < 5 {
        return Ok(());
    }
    let link = &message.text().unwrap()[5..];
    let mut vid_name = get_name(&link).unwrap_or_else(|_| { "unknown_name".to_string() });
    vid_name.push_str(".mp4");
    if let Ok((filename, _dir)) = dwnld_file(link, &vid_name) {
        let vid = telegram_bot::InputFile::from(InputFileUpload::with_path(filename));
        api.send(message.chat.video(vid)).await?;
        api.send(message.delete()).await?;
    } else {
        vid_handle_in_mem(api, message).await?;
    }
    Ok(())
}

pub async fn mus_handle_in_mem(api: Api, message: Message) -> Result<(), Error> {
    if message.text().unwrap().len() < 5 {
        return Ok(());
    }
    let link = &message.text().unwrap()[5..];
    let mut vid_name = get_name(&link).unwrap_or_else(|_| { "unknown_name".to_string() });
    vid_name.push_str(".mp3");
    let aud = Command::new("yt-dlp")
        .args(["-f", "bestaudio[ext=m4a][filesize_approx<=50m]", "-o", "-", &link])
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

pub async fn mus_handle(api: Api, message: Message) -> Result<(), Error> {
    if message.text().unwrap().len() < 5 {
        return Ok(());
    }
    let link = &message.text().unwrap()[5..];
    let mut mus_name = get_name(&link).unwrap_or_else(|_| { "unknown_name".to_string() });
    mus_name.push_str(".mp3");
    if let Ok((filename, _dir)) = dwnld_file(link, &mus_name) {
        let mus = InputFileUpload::with_path(filename);
        api.send(message.chat.audio(mus)).await?;
        api.send(message.delete()).await?;
    } else {
        mus_handle_in_mem(api, message).await?;
    }
    Ok(())
}
