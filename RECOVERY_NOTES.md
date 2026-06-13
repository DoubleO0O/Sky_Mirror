# SKY recovered repository snapshot

This repository was reconstructed on Arch Linux after the transferred working tree was found without `.git`.

Known facts:

* Expected historical baseline: `9348b21 chore: establish Phase 45.6 baseline`
* The original Git history was not present in `/home/double/sky_mirror` at recovery time.
* Phase 45–47R source content was already present in the transferred working tree.
* Phase 47 Linux validation passed on Arch:

  * `cargo check --features smithay-linux`
  * `cargo test --features smithay-linux`
* Phase 48A Real Smithay Adapter Skeleton was implemented and accepted on Arch.
* This repository starts from the accepted recovered source snapshot.
* This commit must not be represented as the original historical baseline.

Important boundaries:

* `supports_real_wayland_surfaces` remains false.
* `supports_gpu_rendering` remains false.
* No real `wl_surface`, `xdg_toplevel`, DRM, GBM, libinput, udev, X11, Vulkan, GPU rendering, or compositor event loop is implemented.
* Phase 48A is only a real Smithay adapter skeleton.
