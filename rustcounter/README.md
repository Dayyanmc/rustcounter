# rustcounter

A counter in a Linux kernel module, written in Rust.

Writes to `/dev/rustcounter` increment a number. Reads return the current value.

## Demo

```
$ sudo insmod rustcounter.ko
$ sudo dmesg | tail -1
rustcounter: module loaded

$ sudo cat /dev/rustcounter
0

$ echo hi  | sudo tee /dev/rustcounter > /dev/null
$ echo bye | sudo tee /dev/rustcounter > /dev/null
$ echo foo | sudo tee /dev/rustcounter > /dev/null
$ echo bar | sudo tee /dev/rustcounter > /dev/null
$ sudo cat /dev/rustcounter
4

$ for i in $(seq 1 500); do echo x | sudo tee /dev/rustcounter > /dev/null & done; wait
$ sudo cat /dev/rustcounter
504

$ sudo rmmod rustcounter
$ sudo dmesg | tail -1
rustcounter: module unloaded, final = 504
```

## What it does

A misc char device backed by an `AtomicU64`. Every `write(2)` syscall adds one to the counter. Every `read(2)` returns the counter as ASCII text. Because the counter is atomic, parallel writers from different CPUs can't lose increments.

## Build & Run

In an Ubuntu 26.04 VM with the Rust-for-Linux toolchain installed:

```bash
sudo apt update
sudo apt install -y build-essential linux-headers-$(uname -r) kmod \
                    rustc-1.93 rust-1.93-src bindgen
sudo update-alternatives --install /usr/bin/rustc rustc /usr/bin/rustc-1.93 100

make
sudo insmod rustcounter.ko
sudo cat /dev/rustcounter
echo bump | sudo tee /dev/rustcounter > /dev/null
sudo rmmod rustcounter
```

## Code tour

Everything is in `rustcounter.rs`.

Lines 20 and 21 are the whole module state: a `u64` counter and a `bool` flag for read tracking. Lines 29-37 register `/dev/rustcounter` with the kernel. Lines 39-43 print the final value to dmesg on rmmod.

The actual logic is in `write_iter` (55-65) and `read_iter` (67-75). Writes drain the user's bytes into a `KVec`, bump the counter, and reset the read flag. Reads check the flag, format the counter as text, and copy it out. If the flag is already set, the read returns 0 and `cat` sees EOF.

## Design notes

I used `AtomicU64` instead of a `Mutex` because the only operations on the counter are "add one" and "read". Atomics do both without a critical section, so there's no point taking a lock.

Writes throw away the bytes you sent. The spec says "increment on every write", which I read as per-syscall. So `echo hi` and `dd bs=1M count=1` both add exactly one. The bytes still get copied in because that's what the kernel expects, but they go into a temporary `KVec` that's freed at the end of the function.

The `CONSUMED` flag is what makes `cat` work. Without it, every call to `read(2)` would re-emit the value, and `cat` would loop forever. Setting it on first read and returning 0 after that gives `cat` an EOF. Writes clear the flag so the next read sees a fresh value.

In C, `static u64 count; count++;` on the write path races on SMP. Rust doesn't even let you compile that: mutating a `static` without a sync primitive is a type error.

## Future work

- Reset via ioctl. Right now you have to rmmod/insmod to zero the counter.
- A `/proc/rustcounter` entry exposing write count and read count separately.
- Per-open counters stored on the device pointer, so different processes can keep separate counts through one driver.

## License

GPL-2.0, to match the Linux kernel. See LICENSE.
