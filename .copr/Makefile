# COPR "make_srpm" entry point. COPR clones the repo at the pushed commit and
# runs, as root inside a Fedora chroot with network access:
#   make -f <repo>/.copr/Makefile srpm outdir=<dir> spec=<spec>
# The real work is in packaging/fedora/make-srpm.sh.
top := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
outdir ?= $(CURDIR)

.PHONY: srpm
srpm:
	dnf install -y cargo git-core rpm-build xz
	bash $(top)/packaging/fedora/make-srpm.sh $(outdir)
