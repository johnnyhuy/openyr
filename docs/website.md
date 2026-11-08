# Project website

The GitHub Pages site lives in `site/`. It is plain HTML, CSS and JavaScript;
there is no package manager, build step, external font or analytics service.
The original CSS uses a classic command-console layout without copied logos.
The gallery references the same commit-pinned captures as the README. Retail
art is hosted outside the source tree and keeps its original ownership.

Run `mise run site:serve`, then open http://localhost:8000. The initial screenshot,
page navigation, download and installation instructions work without JavaScript.
JavaScript enables explicit capture selection; GIFs load only when selected.
Source ZIP downloads are available. No prebuilt application is currently
published; do not advertise a binary download until one is actually available.

`.github/workflows/pages.yml` uploads only `site/` and deploys through the
`github-pages` environment after a main-branch website change or manual dispatch.
The repository owner must first set Settings > Pages > Source to GitHub Actions.
The connected app returned HTTP 403 for first-time setup, and configure-pages
explicitly requires an owner token other than GITHUB_TOKEN for enablement.
After that one-time setting, normal deployments use the scoped workflow token. The public URL is
https://johnnyhuy.github.io/openyr/. Use relative paths for local CSS/JavaScript
so previews and the project-site subdirectory both work.

Review desktop/mobile layout, keyboard focus, all five gallery controls,
no-JavaScript fallback, media error recovery and public media URLs when changing
the site. Browser proof may mirror the verified public capture bytes locally
when the cloud Chromium TLS route is unavailable; record that distinction.
