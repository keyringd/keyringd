use crate::core::model::Workspace;
use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const VIDEO_NAME: &str = "keyringd-in-60-seconds.mp4";
const POSTER_NAME: &str = "keyringd-in-60-seconds-poster.png";
const MAX_UPLOAD_BYTES: u64 = 16 * 1024 * 1024;
const RENDER_TIMEOUT: Duration = Duration::from_secs(65);

pub(in crate::core) struct PromoVideoRenderer {
    website: PathBuf,
    source: PathBuf,
    output: PathBuf,
}

impl PromoVideoRenderer {
    pub(in crate::core) fn new(workspace: &Workspace) -> Self {
        let website = workspace.root().join("website");
        Self {
            source: website.join("tools/promo-video"),
            output: website.join("target/promo-video"),
            website,
        }
    }

    pub(in crate::core) fn build(&self) -> Result<()> {
        ensure!(
            self.source.join("film.html").is_file(),
            "promo-video source is missing: {}",
            self.source.display()
        );
        if self.output.exists() {
            fs::remove_dir_all(&self.output).with_context(|| {
                format!(
                    "could not clear promo-video output: {}",
                    self.output.display()
                )
            })?;
        }
        fs::create_dir_all(&self.output)?;

        let server = Server::http(("127.0.0.1", 0)).map_err(|error| {
            anyhow::anyhow!("could not start the promo-video localhost server: {error}")
        })?;
        let address = server.server_addr();
        let profile = self.output.join("chrome-profile");
        let mut chrome = Command::new(chrome_executable()?)
            .args([
                "--headless",
                "--disable-gpu",
                "--disable-background-networking",
                "--no-first-run",
                "--disable-breakpad",
                "--disable-crash-reporter",
                "--disable-crashpad-for-testing",
            ])
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg(format!("http://{address}"))
            .current_dir(&self.source)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("could not start Google Chrome; set CHROME_PATH to its executable")?;

        let rendered = self.receive_until_complete(&server, &mut chrome);
        let _ = chrome.kill();
        let _ = chrome.wait();
        rendered?;
        self.publish()
    }

    fn receive_until_complete(&self, server: &Server, chrome: &mut Child) -> Result<()> {
        let started = Instant::now();
        loop {
            if started.elapsed() >= RENDER_TIMEOUT {
                bail!("timed out while rendering or verifying the promo video");
            }
            if let Some(status) = chrome.try_wait()? {
                bail!("Google Chrome exited before rendering completed: {status}");
            }
            if let Some(request) = server.recv_timeout(Duration::from_millis(25))?
                && self.handle_request(request)?
            {
                return Ok(());
            }
        }
    }

    fn handle_request(&self, mut request: Request) -> Result<bool> {
        let method = request.method().clone();
        let target = request.url().to_owned();
        match (&method, target.as_str()) {
            (&Method::Get, "/") => {
                self.send_file(request, &self.source.join("film.html"), "text/html")?
            }
            (&Method::Get, "/film.js") => {
                self.send_file(request, &self.source.join("film.js"), "text/javascript")?
            }
            (&Method::Get, "/logo.svg") => self.send_file(
                request,
                &self.website.join("public/icon.svg"),
                "image/svg+xml",
            )?,
            (&Method::Get, "/assets/person-normal-flat.png") => self.send_file(
                request,
                &self.source.join("assets/person-normal-flat.png"),
                "image/png",
            )?,
            (&Method::Get, "/assets/person-worried-flat.png") => self.send_file(
                request,
                &self.source.join("assets/person-worried-flat.png"),
                "image/png",
            )?,
            (&Method::Get, "/assets/attacker-hooded-flat.png") => self.send_file(
                request,
                &self.source.join("assets/attacker-hooded-flat.png"),
                "image/png",
            )?,
            (&Method::Post, "/error") => {
                let body = read_body(&mut request)?;
                request.respond(Response::empty(StatusCode(200)))?;
                bail!(
                    "promo-video page failed: {}",
                    String::from_utf8_lossy(&body)
                );
            }
            (&Method::Post, target) if target.starts_with("/save/") => {
                let name = &target[6..];
                ensure!(
                    matches!(
                        name,
                        VIDEO_NAME | POSTER_NAME | "contact-sheet.png" | "verification.json"
                    ),
                    "promo-video page tried to save an unexpected file: {name}"
                );
                let body = read_body(&mut request)?;
                fs::write(self.output.join(name), &body)?;
                if name == "verification.json" {
                    println!("{name} {} {}", body.len(), String::from_utf8_lossy(&body));
                } else {
                    println!("{name} {}", body.len());
                }
                request.respond(Response::empty(StatusCode(200)))?;
                return Ok(name == "verification.json");
            }
            _ => request.respond(Response::from_string("Not found").with_status_code(404))?,
        }
        Ok(false)
    }

    fn send_file(&self, request: Request, path: &Path, content_type: &str) -> Result<()> {
        let body = fs::read(path)
            .with_context(|| format!("could not read promo-video input: {}", path.display()))?;
        let header = Header::from_bytes("Content-Type", content_type)
            .map_err(|_| anyhow::anyhow!("invalid promo-video response content type"))?;
        request.respond(Response::from_data(body).with_header(header))?;
        Ok(())
    }

    fn publish(&self) -> Result<()> {
        verify_metadata(&fs::read(self.output.join("verification.json"))?)?;
        let video = self.output.join(VIDEO_NAME);
        let poster = self.output.join(POSTER_NAME);
        ensure!(
            video.is_file(),
            "rendered video is missing: {}",
            video.display()
        );
        ensure!(
            poster.is_file(),
            "rendered poster is missing: {}",
            poster.display()
        );

        let media = self.website.join("public/media");
        fs::create_dir_all(&media)?;
        fs::copy(&video, media.join(VIDEO_NAME))?;
        fs::copy(&poster, media.join(POSTER_NAME))?;

        let version = hex_digest(&fs::read(&video)?)[..12].to_owned();
        let page = self.website.join("content/keyringd-in-60-seconds.html");
        let source = fs::read_to_string(&page)?;
        let source = replace_asset_version(&source, POSTER_NAME, &version)?;
        let source = replace_asset_version(&source, VIDEO_NAME, &version)?;
        fs::write(&page, source)?;

        println!("published {}", media.join(VIDEO_NAME).display());
        println!("published {}", media.join(POSTER_NAME).display());
        Ok(())
    }
}

fn read_body(request: &mut Request) -> Result<Vec<u8>> {
    let length = request
        .body_length()
        .context("POST request is missing Content-Length")? as u64;
    ensure!(length <= MAX_UPLOAD_BYTES, "HTTP request body is too large");
    let mut body = Vec::with_capacity(length as usize);
    request
        .as_reader()
        .take(length + 1)
        .read_to_end(&mut body)?;
    ensure!(
        body.len() as u64 == length,
        "HTTP request body length is invalid"
    );
    Ok(body)
}

fn verify_metadata(bytes: &[u8]) -> Result<()> {
    let metadata: serde_json::Value = serde_json::from_slice(bytes)?;
    ensure!(
        metadata["duration"].as_f64() == Some(60.0),
        "video duration is not 60 seconds"
    );
    ensure!(
        metadata["width"].as_u64() == Some(1280),
        "video width is not 1280 pixels"
    );
    ensure!(
        metadata["height"].as_u64() == Some(720),
        "video height is not 720 pixels"
    );
    ensure!(
        metadata["frames"].as_u64() == Some(1440),
        "video does not contain 1440 frames"
    );
    Ok(())
}

fn replace_asset_version(source: &str, asset: &str, version: &str) -> Result<String> {
    let prefix = format!("{{site.url.resolve('media/{asset}')}}?v=");
    let start = source
        .find(&prefix)
        .with_context(|| format!("cache marker for {asset} is missing"))?;
    ensure!(
        source[start + prefix.len()..].find(&prefix).is_none(),
        "cache marker for {asset} occurs more than once"
    );
    let value_start = start + prefix.len();
    let value_end = value_start
        + source[value_start..]
            .find('"')
            .with_context(|| format!("cache marker for {asset} is not terminated"))?;
    let mut updated = source.to_owned();
    updated.replace_range(value_start..value_end, version);
    Ok(updated)
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn chrome_executable() -> Result<PathBuf> {
    if let Some(path) = env::var_os("CHROME_PATH") {
        return Ok(path.into());
    }
    if cfg!(windows) {
        for (variable, suffix) in [
            ("ProgramFiles", "Google/Chrome/Application/chrome.exe"),
            ("ProgramFiles(x86)", "Google/Chrome/Application/chrome.exe"),
            ("LOCALAPPDATA", "Google/Chrome/Application/chrome.exe"),
        ] {
            if let Some(root) = env::var_os(variable) {
                let candidate = PathBuf::from(root).join(suffix);
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }
        bail!("Google Chrome was not found; set CHROME_PATH to its executable");
    }
    if cfg!(target_os = "macos") {
        let candidate =
            PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
        ensure!(
            candidate.is_file(),
            "Google Chrome was not found; set CHROME_PATH to its executable"
        );
        return Ok(candidate);
    }
    Ok(PathBuf::from("google-chrome"))
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "promo video" {
            case "Updates only the selected asset cache marker" {
                verify "replace the version while preserving the surrounding Qute expression" {
                    let source = "poster=\"{site.url.resolve('media/keyringd-in-60-seconds-poster.png')}?v=old\"";
                    let updated = replace_asset_version(source, POSTER_NAME, "abc123").unwrap();
                    assert_eq!(updated, "poster=\"{site.url.resolve('media/keyringd-in-60-seconds-poster.png')}?v=abc123\"");
                }
            }

            case "Rejects incomplete render metadata" {
                verify "require the duration, dimensions and frame count" {
                    assert!(verify_metadata(br#"{"duration":60,"width":1280,"height":720,"frames":1440}"#).is_ok());
                    assert!(verify_metadata(br#"{"duration":59,"width":1280,"height":720,"frames":1440}"#).is_err());
                }
            }

        }
    }
}
