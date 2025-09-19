CARGO_TOML := Cargo.toml

# Extract version from Cargo.toml
define get_version
$(shell grep -m1 '^version' $(CARGO_TOML) | sed 's/.*"\(.*\)".*/\1/')
endef

# Update version in Cargo.toml
define update_version
	sed -i 's/^version = ".*"/version = "$(1)"/' $(CARGO_TOML)
endef

.PHONY: create_tag update-patch update-minor update-major prepare-release release release-patch release-minor release-major

# Version update targets
update-patch:
	@echo "Updating patch version..."
	$(eval CURRENT_VERSION := $(call get_version))
	$(eval NEW_VERSION := $(shell echo $(CURRENT_VERSION) | awk -F. '{$$3=$$3+1} 1' OFS=.))
	$(call update_version,$(NEW_VERSION))
	@echo "Version updated from $(CURRENT_VERSION) to $(NEW_VERSION)"

update-minor:
	@echo "Updating minor version..."
	$(eval CURRENT_VERSION := $(call get_version))
	$(eval NEW_VERSION := $(shell echo $(CURRENT_VERSION) | awk -F. '{$$2=$$2+1; $$3=0} 1' OFS=.))
	$(call update_version,$(NEW_VERSION))
	@echo "Version updated from $(CURRENT_VERSION) to $(NEW_VERSION)"

update-major:
	@echo "Updating major version..."
	$(eval CURRENT_VERSION := $(call get_version))
	$(eval NEW_VERSION := $(shell echo $(CURRENT_VERSION) | awk -F. '{$$1=$$1+1; $$2=0; $$3=0} 1' OFS=.))
	$(call update_version,$(NEW_VERSION))
	@echo "Version updated from $(CURRENT_VERSION) to $(NEW_VERSION)"

# Prepare release: update Cargo.lock without --locked flag
prepare-release:
	@echo "Preparing release..."
	$(eval VERSION := $(call get_version))
	@echo "Current version: $(VERSION)"
	@echo "Updating Cargo.lock..."
	cargo update
	@echo "Building project to verify changes..."
	cargo build
	@echo "Running tests to ensure stability..."
	cargo test
	@echo "Release preparation complete for version $(VERSION)"

# Prepare version update (on feature branch)
version-commit: prepare-release
	@echo "Committing version changes..."
	$(eval VERSION := $(call get_version))
	$(eval BRANCH := $(shell git branch --show-current))
	git add Cargo.toml Cargo.lock
	git commit -m "Update version to $(VERSION)"
	git push origin $(BRANCH)
	@echo "✅ Version $(VERSION) committed to branch $(BRANCH)"
	@echo "📝 Next steps:"
	@echo "   1. Create PR: $(BRANCH) → main"
	@echo "   2. Merge PR to get version into main"
	@echo "   3. Run 'make release' to trigger auto-tagging"

# Release: create tag directly from main (simplified workflow)
release:
	@echo "Creating release from main branch..."
	$(eval VERSION := $(call get_version))
	$(eval CURRENT_BRANCH := $(shell git branch --show-current))
	@if [ "$(CURRENT_BRANCH)" != "main" ]; then \
		echo "ERROR: Release must be run from main branch. Current branch: $(CURRENT_BRANCH)"; \
		echo "Switch to main: git checkout main && git pull origin main"; \
		exit 1; \
	fi
	@echo "Checking if working directory is clean..."
	@if ! git diff-index --quiet HEAD --; then \
		echo "ERROR: Working directory has uncommitted changes. Please commit or stash them."; \
		exit 1; \
	fi
	@echo "Verifying version $(VERSION) is ready for release..."
	@echo "Building and testing to ensure stability..."
	cargo build --release
	cargo test
	@echo "Creating release tag v$(VERSION)..."
	git tag -a v$(VERSION) -m "Release version v$(VERSION)"
	git push origin v$(VERSION)
	@echo "✅ Release v$(VERSION) completed!"
	@echo "🚀 GitHub Actions will build and publish the release"
	@echo "📝 View release: https://github.com/Reqvire/reqvire/releases/tag/v$(VERSION)"

# Manual tag creation (backup method)
release-tag:
	@echo "Manual tag creation (backup method)..."
	$(eval VERSION := $(call get_version))
	$(eval BRANCH := $(shell git branch --show-current))
	@if [ "$(BRANCH)" != "releases" ]; then \
		echo "ERROR: Tags should be created from releases branch. Current branch: $(BRANCH)"; \
		echo "Switch to releases: git checkout releases"; \
		exit 1; \
	fi
	git tag -a v$(VERSION) -m "Release version v$(VERSION)"
	git push origin v$(VERSION)
	@echo "Release v$(VERSION) tagged and pushed from releases!"

create_tag:
	@echo "Creating tag..."
	$(eval VERSION := $(call get_version))
	@echo "New version: $(VERSION)"
	git tag -a v$(VERSION) -m "Release version v$(VERSION)"
	git push origin v$(VERSION)

# Automated release preparation (creates branch, updates version, commits, creates PR)
release-patch:
	@echo "🚀 Starting automated patch release process..."
	$(eval CURRENT_VERSION := $(call get_version))
	$(eval NEW_VERSION := $(shell echo $(CURRENT_VERSION) | awk -F. '{$$3=$$3+1} 1' OFS=.))
	$(eval BRANCH_NAME := release/v$(NEW_VERSION))
	@echo "Creating release branch: $(BRANCH_NAME)"
	git checkout -b $(BRANCH_NAME)
	@echo "Updating version from $(CURRENT_VERSION) to $(NEW_VERSION)..."
	$(call update_version,$(NEW_VERSION))
	@$(MAKE) version-commit
	@echo "Creating pull request..."
	@if command -v gh >/dev/null 2>&1; then \
		gh pr create --base main --head $(BRANCH_NAME) \
			--title "Update version to v$(NEW_VERSION)" \
			--body "Automated version bump for release v$(NEW_VERSION)"; \
		echo "✅ Pull request created successfully!"; \
		echo "📝 Next steps:"; \
		echo "   1. Review and merge the PR"; \
		echo "   2. Run 'git checkout main && git pull origin main'"; \
		echo "   3. Run 'make release' to create the tag and trigger automated release"; \
	else \
		echo "⚠️  GitHub CLI (gh) not found. Please create PR manually:"; \
		echo "   Branch: $(BRANCH_NAME) → main"; \
		echo "   Title: Update version to v$(NEW_VERSION)"; \
	fi

release-minor:
	@echo "🚀 Starting automated minor release process..."
	$(eval CURRENT_VERSION := $(call get_version))
	$(eval NEW_VERSION := $(shell echo $(CURRENT_VERSION) | awk -F. '{$$2=$$2+1; $$3=0} 1' OFS=.))
	$(eval BRANCH_NAME := release/v$(NEW_VERSION))
	@echo "Creating release branch: $(BRANCH_NAME)"
	git checkout -b $(BRANCH_NAME)
	@echo "Updating version from $(CURRENT_VERSION) to $(NEW_VERSION)..."
	$(call update_version,$(NEW_VERSION))
	@$(MAKE) version-commit
	@echo "Creating pull request..."
	@if command -v gh >/dev/null 2>&1; then \
		gh pr create --base main --head $(BRANCH_NAME) \
			--title "Update version to v$(NEW_VERSION)" \
			--body "Automated version bump for release v$(NEW_VERSION)"; \
		echo "✅ Pull request created successfully!"; \
		echo "📝 Next steps:"; \
		echo "   1. Review and merge the PR"; \
		echo "   2. Run 'git checkout main && git pull origin main'"; \
		echo "   3. Run 'make release' to create the tag and trigger automated release"; \
	else \
		echo "⚠️  GitHub CLI (gh) not found. Please create PR manually:"; \
		echo "   Branch: $(BRANCH_NAME) → main"; \
		echo "   Title: Update version to v$(NEW_VERSION)"; \
	fi

release-major:
	@echo "🚀 Starting automated major release process..."
	$(eval CURRENT_VERSION := $(call get_version))
	$(eval NEW_VERSION := $(shell echo $(CURRENT_VERSION) | awk -F. '{$$1=$$1+1; $$2=0; $$3=0} 1' OFS=.))
	$(eval BRANCH_NAME := release/v$(NEW_VERSION))
	@echo "Creating release branch: $(BRANCH_NAME)"
	git checkout -b $(BRANCH_NAME)
	@echo "Updating version from $(CURRENT_VERSION) to $(NEW_VERSION)..."
	$(call update_version,$(NEW_VERSION))
	@$(MAKE) version-commit
	@echo "Creating pull request..."
	@if command -v gh >/dev/null 2>&1; then \
		gh pr create --base main --head $(BRANCH_NAME) \
			--title "Update version to v$(NEW_VERSION)" \
			--body "Automated version bump for release v$(NEW_VERSION)"; \
		echo "✅ Pull request created successfully!"; \
		echo "📝 Next steps:"; \
		echo "   1. Review and merge the PR"; \
		echo "   2. Run 'git checkout main && git pull origin main'"; \
		echo "   3. Run 'make release' to create the tag and trigger automated release"; \
	else \
		echo "⚠️  GitHub CLI (gh) not found. Please create PR manually:"; \
		echo "   Branch: $(BRANCH_NAME) → main"; \
		echo "   Title: Update version to v$(NEW_VERSION)"; \
	fi


