# Loading fidelity proof

Source commit 3695f0dd69decfad4444674939c26d49af8080de. The final merge tree drops this directory before a
squash merge. These retail-derived captures are PR-branch evidence only.
The release upload endpoint returned HTTP 401 earlier in this session, so this
uses the authorized branch fallback.

The original reference is a public CodeWeavers screenshot:
https://www.codeweavers.com/compatibility/crossover/command-and-conquer-red-alert-2-yuris-revenge
https://media.codeweavers.com/pub/crossover/website/appdb/255644f70a7152708e96c9dd87e773e2.png
SHA256 c803596685de291f2d50aa36ad3762140c3a49fbdb30b43a4dc2de5f3e8da768.
It uses Yuri as a static reference, without enabling Yuri gameplay. The shared
layout fixture is a compositor regression, not a live Yuri match. Actual live
pointer input selects supported Russia in retail registry order and enters EB1.

Five original glyph masks match across 16638 samples (3569 foreground pixels),
and 716 badge pixels match exactly after RGB565 normalization. The independent
measure-reference.py accepts the public PNG and YRII.PCX from the user's install;
no original implementation or memory is inspected. The derived receipts and
source tests retain the measurement, without shipping either asset on main.

Live Xorg screenshots and a 10-second 60 fps recording succeeded. Native PNG
compression is lossless; decoded-pixel hashes match before and after compression.
The default EB1 run reaches Lost and tick-6000 hash 752f3f10fc2821e5.
Runtime progress timing, preview sampling, backing/style differences, physical
Mac Retina and audible playback remain unverified. See docs/formats/loading-screen.md.
