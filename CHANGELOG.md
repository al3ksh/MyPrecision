# Changelog

Each release's section becomes its GitHub Release notes and the notes the
in-app updater shows. `npm run release` refuses to publish a version that has
no section here.

## Unreleased

- The app is now MyStation, and takes the name of the laptop it runs on:
  MyPrecision, MyXPS or MyLatitude. Updating from MyPrecision moves the
  sign-in task over and removes the old install. Settings and history are
  kept.

## 0.4.0 — 2026-10-12

- Battery profiles are now called Desk and Remote.
- Choose integrated or NVIDIA graphics per app on the Power page.
- The tray icon shows up reliably after boot.
- Fixes to the flyout strip, sensor dot spacing and the thermal tile label.

## 0.3.0 — 2026-10-09

- Hardware page: drive health and wear forecast, startup apps and boot
  times, and USB devices with the battery draw each one adds.
- Power page: which apps keep the NVIDIA GPU awake, battery use by app and
  sleep drain.
- BIOS settings page through Dell Command | Configure.
- Links to the Dell Command downloads when they are missing.
- The tray flyout opens at once.

## 0.2.0 — 2026-10-09

- Thermal mode by power source, a battery profile schedule and protection
  when the laptop stays plugged in for a long time.
- Open the Dell warranty and support page for the service tag.
- Windows show only once their data has loaded, so content no longer pops in.

## 0.1.0 — 2026-10-09

- First release: tray flyout and full window with battery charge limits,
  thermal modes, battery health and charts, and signed in-app updates.
