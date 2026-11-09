# Side archive naming proof

Source e4ecd9a8027efbfc9dcd985ce8834e24bd8a2cf5, tree f978273eb0a1bbe83ceda8e3b2a0e3691193f062.

The two PNGs are supplied retail-derived captures from the original PR #310 author's draft release. They were downloaded through release asset IDs 727490643 and 727490639. Their actual bytes match the API size/SHA256 metadata in author-release-metadata.json. bkgdlgy.png is 856x736; radary.png is the supplied 1008x660 montage. These are provided static captures, not newly decoded retail art from this cloud build. original-author-report.json preserves the initial reported scan/decode results and source commit. Retail data is absent here; those reports were not rerun or converted into cloud retail passes.

The current source's locked workspace test, strict clippy, fmt and three Python provenance checks passed. The new asset-independent literal-ID test resolves all ten added names. The optional retail test self-skips without a local install.

The terminal recording runs the actual isolated release mix-dump through scripts/proof_build.py. make-fixtures.py authors two classic MIX files with the recorded literal entry IDs and plain text payloads. verify-cli.py checks all seven Yuri and three Allied name mappings against actual CLI output. The fixtures test metadata naming, not SHP/palette decoding, archive provenance or retail completeness. No original asset payloads or engine implementation are used in these fixtures.

No compiled executable is uploaded. The release receipt records its size/hash and exact clean source tree. Provided PNG integrity, authored CLI proof and original author retail reports are distinct evidence. Yuri gameplay, live sidebar wiring, original runtime timing, physical Mac/Retina and audio remain unverified by this slice.

Known blocked original/Mac capture and HTTP401 release-upload paths were not retried. This uses the authorized pinned branch fallback for public review. All proof files must be removed before a source-only squash merge; the original draft release remains unchanged.
