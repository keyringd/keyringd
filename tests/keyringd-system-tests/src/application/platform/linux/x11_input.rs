use anyhow::{Result, ensure};

use super::ScreenPoint;
use crate::environment::linux::Environment;

pub(crate) struct X11Input;

impl X11Input {
    pub(crate) async fn press_key_sequence(
        environment: &Environment,
        window_name: &str,
        keys: &[&str],
    ) -> Result<()> {
        Self::run(
            environment,
            &[
                "search",
                "--onlyvisible",
                "--limit",
                "1",
                "--name",
                window_name,
                "windowfocus",
                "--sync",
            ],
        )
        .await?;
        // Keep the search's window stack out of key delivery. Escape can destroy
        // that window before KeyRelease; XTEST must release into the current focus.
        let mut arguments = Vec::with_capacity(keys.len() * 5);
        for key in keys {
            arguments.extend([
                "key".to_owned(),
                "--clearmodifiers".to_owned(),
                (*key).to_owned(),
                "sleep".to_owned(),
                "0.05".to_owned(),
            ]);
        }
        let arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        Self::run(environment, &arguments).await
    }

    pub(crate) async fn click_then_press_key(
        environment: &Environment,
        point: ScreenPoint,
        key: &str,
    ) -> Result<()> {
        let x = point.x.to_string();
        let y = point.y.to_string();
        Self::run(
            environment,
            &[
                "mousemove",
                &x,
                &y,
                "click",
                "1",
                "key",
                "--clearmodifiers",
                key,
            ],
        )
        .await
    }

    pub(crate) async fn drag(
        environment: &Environment,
        from: ScreenPoint,
        to: ScreenPoint,
    ) -> Result<()> {
        const STEPS: i32 = 8;

        let mut arguments = vec![
            "mousemove".to_owned(),
            from.x.to_string(),
            from.y.to_string(),
            "mousedown".to_owned(),
            "1".to_owned(),
            "sleep".to_owned(),
            "0.3".to_owned(),
        ];
        for step in 1..=STEPS {
            arguments.extend([
                "mousemove".to_owned(),
                (from.x + (to.x - from.x) * step / STEPS).to_string(),
                (from.y + (to.y - from.y) * step / STEPS).to_string(),
                "sleep".to_owned(),
                "0.05".to_owned(),
            ]);
        }
        arguments.extend([
            "sleep".to_owned(),
            "0.2".to_owned(),
            "mouseup".to_owned(),
            "1".to_owned(),
        ]);
        let arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        Self::run(environment, &arguments).await
    }

    async fn run(environment: &Environment, arguments: &[&str]) -> Result<()> {
        let output = environment.run("xdotool", arguments).await?;
        ensure!(output.status.success(), "xdotool failed: {}", output.stderr);
        Ok(())
    }
}
