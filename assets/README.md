# assets

What RSC carries besides its code: its logo and icons, the icons' font, the licence notices.

| File | Role |
|---|---|
| `rsc.svg` | RSC's logo: a serial connector drawn by hand, a signal on it, on a gradient tile |
| `rsc-256.png` | the window's icon, drawn from the logo |
| `rsc.ico` | the Windows executable's icon (16 to 256 pixels), put into `rsc.exe` by `build.rs` |
| `rsc.icns` | the icon of `RSC.app` under macOS |
| `linux/rsc-<size>.png` | the Linux icons, 16 to 256 pixels, that `rsc.desktop` names |
| `rsc.desktop` | RSC's entry in the Linux menus |
| `linux/install.sh` | installs a Linux build for the user, with its menu entry and icon; `--uninstall` takes it away |
| `make_icons.py` | draws `rsc.svg`, `rsc-256.png` and `rsc.ico` (Pillow needed) |
| `make_platform_icons.py` | takes `linux/rsc-<size>.png` and `rsc.icns` from `rsc.ico`, with Python alone |
| `fonts/` | the icons' font (Lucide), its licence and where it comes from |
| `licences/` | the licence record RSC carries (`licences.json`), its text (`THIRD-PARTY-LICENSES.txt`), the standard licence texts |

To draw the logo again, then the icons of each system:

```bash
python3 assets/make_icons.py
python3 assets/make_platform_icons.py
```
