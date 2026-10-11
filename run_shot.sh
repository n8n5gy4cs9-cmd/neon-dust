#!/bin/sh
# usage: ./run_shot.sh out.png secs [game args...]  (software Vulkan, headless)
out=$1; secs=$2; shift 2
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json xvfb-run -a -s "-screen 0 1280x720x24" timeout 300 ./target/${PROFILE:-release}/neon-dust --shot "$out" "$secs" "$@" 2>&1 | grep -E "ERROR|panicked|WARN.*(wgpu|naga|shader)" | head -20
