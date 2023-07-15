build: deps
	pnpm build
build_release: deps
	pnpm build
build_release_%: deps
	pnpm --filter $* build
build_%: deps
	pnpm --filter $* build

test: deps
	pnpm test
test_%: deps
	pnpm --filter $* test

lint: deps
	pnpm lint

clean: deps
	pnpm clean
clean_%: deps
	pnpm --filter $* clean

very_clean:
	find . -type d -name "node_modules" -exec rm -rf {} +

deps:
	pnpm install

.PHONY: \
	build build_release build_release_% \
	test test_% lint lint_% clean very_clean deps
