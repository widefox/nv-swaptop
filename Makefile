PREFIX   ?= /usr/local
BINDIR   ?= $(PREFIX)/bin
MANDIR   ?= $(PREFIX)/share/man/man1

BINARY   := target/release/nv-swaptop
MANPAGE  := $(shell find target/release/build -name 'nv-swaptop.1' -path '*/out/*' 2>/dev/null | head -1)

.PHONY: build install uninstall

build:
	cargo build --release

install: build
	@MANPAGE="$$(find target/release/build -name 'nv-swaptop.1' -path '*/out/*' 2>/dev/null | head -1)"; \
	if [ -z "$$MANPAGE" ]; then echo "error: manpage not found in build output" >&2; exit 1; fi; \
	install -Dm755 $(BINARY) $(DESTDIR)$(BINDIR)/nv-swaptop; \
	install -Dm644 "$$MANPAGE" $(DESTDIR)$(MANDIR)/nv-swaptop.1

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/nv-swaptop
	rm -f $(DESTDIR)$(MANDIR)/nv-swaptop.1
