# Fresh front-match save target

Source 2d23672c6716027d01c8a9fc3023b17acb069ee5, tree 5ade32ea5b9c0d8e44be2c2efd90c1953934de18.

Authored native Session proof for #306. The test drives normal return-to-front code, controlled ordinary prepared-result channels and actual F5/F9 keyboard handlers. It does not execute the retail asset loader or reconstruct a saved match from disk.

Four actual shared-compositor PNG/JSON states show the earlier named match, successful fresh front commit, F5 save success and F9 confirmation of quicksave.oyrsave. Screenshots display save basenames. Temporary fixture paths in logs/state receipts refer only to owned scratch files. The earlier named file stays byte-for-byte unchanged while F5 writes the complete exact fresh Session bytes to quicksave in the same folder. The first frame has the earlier Session's tick/hash; the following states retain the new Session's tick/hash. No simulation steps occur during the tested UI transitions.

Three native tests cover all three supported layouts, explicit front/in-match saved-result targets, retained in-match restart targets, Main Menu entry, preview, failure and cancellation. before-regression.rs.txt is the initial seed test on main before the fix; before-fix.log records its actual F5 replacing the earlier named file. The final tests extend that seed with target-policy and exact file-byte checks.

The native debug test executable is separately selected and hashed in the terminal recording. Its synthetic fixture run is distinct from the isolated release receipt and real --help launch. The release smoke establishes clean-tree executable provenance and CLI launch, not execution of debug-only native fixtures.

The GIF shows one exported state per second, not runtime timing. No retail artwork, cursor sheet or physical-window input is supplied. Retail loading/reconstruction, physical Mac/Retina input, audio and original-game parity remain unverified. Known blocked original/Mac captures and HTTP401 release uploads were not retried. Proof uses the authorized pinned branch fallback and must be removed before squash merge.
