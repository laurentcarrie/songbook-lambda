help: ## Show this help
	@grep -E '^[a-zA-Z0-9_-]+:.*##' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-10s %s\n", $$1, $$2}'

s3testpath=s3://zik-laurent/test

# Drum pattern library for the strudel pages. It lives outside the repo, so the
# targets below pass it only when it is actually there: a library that is named
# but missing fails the build on the first pattern it cannot resolve, while no
# library at all is a supported mode - strudel.html is written empty.
drumsdir ?= $(HOME)/work/songbook/drums
drumsopt = $(if $(wildcard $(drumsdir)),--drum-patterns-dir $(drumsdir))

clean: ## Remove delivery and sandbox directories
	rm -rf delivery sandbox

install: ## Install the band-songbook binary into ~/.cargo/bin
	cargo install --path . --bin band-songbook

run: ## Build songs and books from local test data
	cargo run --bin band-songbook -- \
		--songs-srcdir tests/data \
		--books-srcdir tests/data/books \
		--settings tests/data/settings.yml \
		--sandbox sandbox \
		--delivery delivery \
		$(drumsopt)

# Every chord macro of src/resources/texfiles/chords.tex in tables: one table per accidental (natural, flat,
# sharp), a row per letter A..G, a column per kind of chord (see tools/chords_sheet.py). A chord added to
# chords.tex that the script does not know goes in an "others" table. Needs lualatex and the songbook fonts.
chordsdir = sandbox/chords

chords-pdf: ## Generate sandbox/chords/chords.pdf: all the chords drawn by chords.tex, a row per letter
	mkdir -p $(chordsdir)
	cp src/resources/texfiles/chords.tex $(chordsdir)/chords.tex
	python3 tools/chords_sheet.py src/resources/texfiles/chords.tex $(chordsdir)/all-chords.tex
	cd $(chordsdir) && { lualatex -interaction=nonstopmode -halt-on-error all-chords.tex > all-chords.log 2>&1 || { tail -30 all-chords.log; exit 1; }; }
	mv $(chordsdir)/all-chords.pdf $(chordsdir)/chords.pdf
	@echo $(chordsdir)/chords.pdf

open: ## Open strudel HTML for ca_me_vexe
	open sandbox/songs/mademoiselle_K/ca_me_vexe/strudel.html

s3: ## Build rasmus song from S3
	aws s3 rm --recursive $(s3testpath)	
	cargo run --bin band-songbook -- \
		--songs-srcdir s3://zik-laurent/songs \
		--settings s3://zik-laurent/songs/settings.yml \
		--sandbox sandbox \
		--delivery $(s3testpath)/delivery \
		--pattern rasmus \
		--drum-patterns-dir s3://zik-laurent/drums

run-all: ## Build songs from local
	cargo run --bin band-songbook -- \
		--songs-srcdir ../../songbook/songs \
		--settings ../../songbook/songs/settings.yml \
		--sandbox sandbox \
		--delivery delivery \
		--pattern u2

refresh-and-reset: ## Reset work branch to main (fetch, reset, force push)
	@branch=$$(git rev-parse --abbrev-ref HEAD); \
	if [ "$$branch" != "work" ]; then echo "Error: not on work branch (on $$branch)"; exit 1; fi
	@if [ -n "$$(git status --porcelain)" ]; then echo "Error: uncommitted changes"; git status --short; exit 1; fi
	git fetch origin main:main
	@local=$$(git rev-parse HEAD); remote=$$(git rev-parse main); \
	if [ "$$local" != "$$remote" ]; then echo "Error: work and main differ (work has unmerged commits)"; git log --oneline main..HEAD; exit 1; fi
	git reset --hard main
	git push --force