.PHONY: cargo test iso run

ifeq ($(shell grub-mkrescue -? &>/dev/null; echo $$?),0)
    grub-mkrescue = grub-mkrescue
else
    ifeq ($(shell grub2-mkrescue -? &>/dev/null; echo $$?),0)
        grub-mkrescue = grub2-mkrescue
    else
      $(error "grub-mkrescue is not found.")
    endif
endif

cargo:
	xargo build --release --target x86_64-unknown-intermezzos-gnu --verbose

cargo-debug:
	xargo build --target x86_64-unknown-intermezzos-gnu --verbose

# cargo test fails for some reason, not sure why yet
test:
	cd console && cargo test
	cd interrupts && cargo test
	cd keyboard && cargo test
	cd pic && cargo test

iso: cargo grub.cfg
	mkdir -p target/isofiles/boot/grub
	cp grub.cfg target/isofiles/boot/grub
	cp target/x86_64-unknown-intermezzos-gnu/release/intermezzos target/isofiles/boot/
	$(grub-mkrescue) -o target/os.iso target/isofiles

iso-debug: cargo-debug grub.cfg
	mkdir -p target/isofiles/boot/grub
	cp grub.cfg target/isofiles/boot/grub
	cp target/x86_64-unknown-intermezzos-gnu/debug/intermezzos target/isofiles/boot/
	$(grub-mkrescue) -o target/os-debug.iso target/isofiles

run: iso
	qemu-system-x86_64 -cdrom target/os.iso

run-debug: iso-debug
	qemu-system-x86_64 -cdrom target/os-debug.iso -s -S -d cpu_reset,int -D qemu.log

gdb: cargo-debug
	@gdb "target/x86_64-unknown-intermezzos-gnu/debug/intermezzos" -ex "target remote :1234" -tui -x gdbcmds


clean:
	rm -Rf target
