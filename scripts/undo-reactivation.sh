# shellcheck shell=bash
# Sourced by pixi during activation (see `[activation] scripts` in pixi.toml); not meant
# to be executed. Manifest activation scripts run after the packages' activate.d scripts.
#
# If the environment that launched this activation already had THIS prefix active
# (direnv `pixi shell-hook`, `pixi shell`), the compiler scripts have just prepended their
# flags a second time. They saved the previous, already-correct values, so put those back.
# See docs/pixi-direnv-double-activation.md.
_pixi_outer="CONDA_ENV_SHLVL_${CONDA_SHLVL:-0}_CONDA_PREFIX"
if [ -n "${!_pixi_outer:-}" ] && [ "${!_pixi_outer}" = "${CONDA_PREFIX:-}" ]; then
    # conda-forge compiler activation (_tc_activation): CONDA_BACKUP_<NAME>
    for _pixi_b in $(compgen -e CONDA_BACKUP_); do
        export "${_pixi_b#CONDA_BACKUP_}=${!_pixi_b}"
        unset "$_pixi_b"
    done
    # cuda-nvcc keeps its own backup
    if [ "${NVCC_PREPEND_FLAGS_BACKUP:-UNSET}" != "UNSET" ]; then
        export NVCC_PREPEND_FLAGS="$NVCC_PREPEND_FLAGS_BACKUP"
        export NVCC_PREPEND_FLAGS_BACKUP="UNSET"
    fi
fi
unset _pixi_outer _pixi_b
