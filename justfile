# Default architecture

export ARCH := env("ARCH", "rv64")

# Default Harts

export HARTS := env("HARTS", "4")

# Set TARGET based on ARCH

TARGET := if ARCH == "rv32" { "riscv32imac-unknown-none-elf" } else { "riscv64gc-unknown-none-elf" }

# Set QEMU binary based on ARCH

QEMU := if ARCH == "rv32" { "qemu-system-riscv32" } else { "qemu-system-riscv64" }

# Build the kernel
build:
    cargo build --target {{ TARGET }}

# Kill zombie QEMU instances
kill-qemu:
    pkill {{ QEMU }} || true

# Build and run in QEMU
run: kill-qemu build
    {{ QEMU }} \
        -machine virt \
        -smp {{ HARTS }} \
        -nographic \
        -serial mon:stdio \
        -kernel target/{{ TARGET }}/debug/risky
