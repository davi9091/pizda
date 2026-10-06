Build-time secrets. Everything in this directory except this file is gitignored.

Not used yet. From M6b on, `make image` will require:

- `authorized_keys`: public SSH key(s) allowed to log in as `doge`
- `wifi.env`: access point settings, e.g.

  ```
  WIFI_SSID=dogecar
  WIFI_PASSPHRASE=change-me-to-something-long
  ```
