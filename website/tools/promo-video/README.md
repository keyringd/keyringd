# Promo video generator

This directory contains the editable source for the 60-second introduction video. `film.js` draws the frames with HTML Canvas and encodes them as H.264 with WebCodecs. The xtask command hosts these files, opens the film in headless Google Chrome, receives the generated media, and publishes it only after verifying its duration, dimensions, and frame count.

Run from the repository root:

```sh
cargo xtask docs pv build
```

Google Chrome is required. Set `CHROME_PATH` to the Chrome executable when it is not installed in its standard location.

Intermediate outputs, the review contact sheet, verification metadata, and the temporary Chrome profile are written to `website/target/promo-video/` and remain ignored. A successful build updates:

- `website/public/media/keyringd-in-60-seconds.mp4`
- `website/public/media/keyringd-in-60-seconds-poster.png`
- the media cache identifier in `website/content/keyringd-in-60-seconds.html`

Commit those published files with the generator sources and illustration assets that produced them.
