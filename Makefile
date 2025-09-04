CARGO_TOML := Cargo.toml

# Extract version from Cargo.toml
define get_version
$(shell grep -m1 '^version' $(CARGO_TOML) | sed 's/.*"\(.*\)".*/\1/')
endef

# Update version in Cargo.toml
define update_version
	sed -i 's/^version = ".*"/version = "$(1)"/' $(CARGO_TOML)
endef

.PHONY: create_tag update-patch update-minor update-major prepare-release release

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

# Complete release process
release: prepare-release
	@echo "Committing version changes..."
	$(eval VERSION := $(call get_version))
	git add Cargo.toml Cargo.lock
	git commit -m "Update version to $(VERSION)"
	@echo "Creating and pushing tag..."
	git tag -a v$(VERSION) -m "Release version v$(VERSION)"
	git push origin main
	git push origin v$(VERSION)
	@echo "Release v$(VERSION) completed!"

create_tag:
	@echo "Creating tag..."
	$(eval VERSION := $(call get_version))
	@echo "New version: $(VERSION)"
	git tag -a v$(VERSION) -m "Release version v$(VERSION)"
	git push origin v$(VERSION)


