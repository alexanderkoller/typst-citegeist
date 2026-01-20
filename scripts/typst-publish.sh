#!/usr/bin/env bash

# typst-publish.sh
# A guided workflow for publishing Typst packages to the Universe

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Global flags
# ─────────────────────────────────────────────────────────────────────────────

DRY_RUN=false
PACKAGES_DIR=/tmp/packages

# Package info (set by parse_typst_toml)
PKG_NAME=""
PKG_VERSION=""
PKG_DESCRIPTION=""
PKG_SOURCE_DIR=""

# ─────────────────────────────────────────────────────────────────────────────
# Colors and styling
# ─────────────────────────────────────────────────────────────────────────────

C_RESET=$'\033[0m'
C_BOLD=$'\033[1m'
C_DIM=$'\033[2m'
C_CYAN=$'\033[36m'
C_GREEN=$'\033[32m'
C_YELLOW=$'\033[33m'
C_RED=$'\033[31m'
C_MAGENTA=$'\033[35m'

# ─────────────────────────────────────────────────────────────────────────────
# Helper functions
# ─────────────────────────────────────────────────────────────────────────────

print_header() {
    echo
    echo "${C_CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${C_RESET}"
    echo "${C_BOLD}${C_CYAN}  $*${C_RESET}"
    echo "${C_CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${C_RESET}"
    echo
}

print_step() {
    echo "${C_MAGENTA}▸${C_RESET} ${C_BOLD}$*${C_RESET}"
}

print_success() {
    echo "${C_GREEN}✓${C_RESET} $*"
}

print_error() {
    echo "${C_RED}✗${C_RESET} $*"
}

print_info() {
    echo "${C_DIM}  $*${C_RESET}"
}

print_warning() {
    echo "${C_YELLOW}⚠${C_RESET} $*"
}

print_dry() {
    echo "${C_YELLOW}[DRY RUN]${C_RESET} ${C_DIM}$*${C_RESET}"
}

run_cmd() {
    # Runs a command, or prints it in dry-run mode
    if [[ "$DRY_RUN" == true ]]; then
        print_dry "$*"
        return 0
    else
        eval "$@"
        return $?
    fi
}

prompt_confirm() {
    local message="$1"
    local default="${2:-}"  # Optional: "y" or "n"
    local answer

    while true; do
        # Show prompt with appropriate hint based on default
        if [[ "$default" == "y" ]]; then
            read -r -p "${C_YELLOW}?${C_RESET} $message ${C_DIM}(Y/n)${C_RESET} " answer
        elif [[ "$default" == "n" ]]; then
            read -r -p "${C_YELLOW}?${C_RESET} $message ${C_DIM}(y/N)${C_RESET} " answer
        else
            read -r -p "${C_YELLOW}?${C_RESET} $message ${C_DIM}(y/n)${C_RESET} " answer
        fi

        # Handle Ctrl-C
        if [[ $? -ne 0 ]]; then
            return 2
        fi

        # Handle empty answer with default
        if [[ -z "$answer" && -n "$default" ]]; then
            answer="$default"
        fi

        case "$answer" in
            [Yy]|[Yy][Ee][Ss])
                return 0
                ;;
            [Nn]|[Nn][Oo])
                return 1
                ;;
        esac
    done
}

prompt_input() {
    local message="$1"
    local default="${2:-}"
    local answer

    if [[ -n "$default" ]]; then
        read -r -p "${C_YELLOW}?${C_RESET} $message ${C_DIM}[$default]${C_RESET} " answer
        if [[ $? -ne 0 ]]; then
            return 2  # Ctrl-C pressed
        fi
        if [[ -z "$answer" ]]; then
            echo "$default"
        else
            echo "$answer"
        fi
    else
        read -r -p "${C_YELLOW}?${C_RESET} $message " answer
        if [[ $? -ne 0 ]]; then
            return 2  # Ctrl-C pressed
        fi
        echo "$answer"
    fi
}

spinner() {
    local pid="$1"
    local frames=("⠋" "⠙" "⠹" "⠸" "⠼" "⠴" "⠦" "⠧" "⠇" "⠏")
    local i=0

    while kill -0 "$pid" 2>/dev/null; do
        printf "\r%s%s%s " "${C_CYAN}" "${frames[$i]}" "${C_RESET}"
        i=$(( (i + 1) % ${#frames[@]} ))
        sleep 0.1
    done
    printf "\r"
}

# ─────────────────────────────────────────────────────────────────────────────
# Core functions
# ─────────────────────────────────────────────────────────────────────────────

check_prerequisites() {
    print_step "Checking prerequisites..."

    local missing=()

    for cmd in git gh; do
        if ! command -v "$cmd" &>/dev/null; then
            missing+=("$cmd")
        fi
    done

    if [[ ${#missing[@]} -gt 0 ]]; then
        print_error "Missing required commands: ${missing[*]}"
        return 1
    fi

    print_success "All prerequisites found"
    return 0
}

parse_typst_toml() {
    local toml_path="$1"

    if [[ ! -f "$toml_path" ]]; then
        print_error "typst.toml not found at $toml_path"
        return 1
    fi

    # Extract values from typst.toml
    PKG_NAME=$(grep '^name' "$toml_path" | sed 's/.*= *"\([^"]*\)".*/\1/')
    PKG_VERSION=$(grep '^version' "$toml_path" | sed 's/.*= *"\([^"]*\)".*/\1/')
    PKG_DESCRIPTION=$(grep '^description' "$toml_path" | sed 's/.*= *"\([^"]*\)".*/\1/')

    if [[ -z "$PKG_NAME" || -z "$PKG_VERSION" ]]; then
        print_error "Could not parse name or version from typst.toml"
        return 1
    fi

    return 0
}

select_package() {
    print_step "Scanning for packages..."

    local release_base="./release/preview"

    if [[ ! -d "$release_base" ]]; then
        print_error "No release/preview directory found"
        print_info "Expected structure: ./release/preview/<package-name>/<version>/"
        return 1
    fi

    # Find all package/version directories
    local packages=()
    local package_paths=()

    for pkg_dir in "$release_base"/*/; do
        if [[ ! -d "$pkg_dir" ]]; then
            continue
        fi
        local pkg_name
        pkg_name=$(basename "$pkg_dir")
        for ver_dir in "$pkg_dir"/*/; do
            if [[ ! -d "$ver_dir" ]]; then
                continue
            fi
            if [[ -f "$ver_dir/typst.toml" ]]; then
                local ver
                ver=$(basename "$ver_dir")
                packages+=("$pkg_name@$ver")
                package_paths+=("$ver_dir")
            fi
        done
    done

    # Sort packages by version (descending) so newest is first
    if [[ ${#packages[@]} -gt 1 ]]; then
        # Build array of "index version" pairs for sorting
        local pairs=()
        for i in "${!packages[@]}"; do
            local ver="${packages[$i]##*@}"
            pairs+=("$i $ver")
        done

        # Sort by version descending and reorder arrays
        local sorted_packages=()
        local sorted_paths=()
        while IFS= read -r line; do
            local idx="${line%% *}"
            sorted_packages+=("${packages[$idx]}")
            sorted_paths+=("${package_paths[$idx]}")
        done < <(printf '%s\n' "${pairs[@]}" | sort -t' ' -k2 -V -r)

        packages=("${sorted_packages[@]}")
        package_paths=("${sorted_paths[@]}")
    fi

    if [[ ${#packages[@]} -eq 0 ]]; then
        print_error "No packages found in $release_base"
        print_info "Expected structure: ./release/preview/<package-name>/<version>/typst.toml"
        return 1
    elif [[ ${#packages[@]} -eq 1 ]]; then
        PKG_SOURCE_DIR=$(realpath "${package_paths[0]}")
        print_success "Found package: ${packages[0]}"
    else
        echo
        echo "${C_YELLOW}?${C_RESET} Select a package to publish: ${C_DIM}(↑/↓ to move, Enter to select)${C_RESET}"
        echo

        local selected=0
        local num_packages=${#packages[@]}

        # Function to draw the menu
        draw_menu() {
            local redraw="$1"
            # Move cursor up to redraw (skip on first draw)
            if [[ "$redraw" == "redraw" ]]; then
                printf "\033[%dA" "$num_packages"
            fi

            local i=0
            for pkg in "${packages[@]}"; do
                if [[ $i -eq $selected ]]; then
                    echo "  ${C_CYAN}▸${C_RESET} ${C_BOLD}$pkg${C_RESET}"
                else
                    echo "    ${C_DIM}$pkg${C_RESET}"
                fi
                ((i++))
            done
        }

        # Initial draw
        draw_menu first

        # Save terminal state and switch to raw mode
        local saved_stty
        saved_stty=$(stty -g)
        stty raw -echo

        # Read arrow keys and enter
        while true; do
            # Read a single character
            local key
            key=$(dd bs=1 count=1 2>/dev/null)

            if [[ "$key" == $'\r' || "$key" == $'\n' ]]; then
                # Enter key
                break
            elif [[ "$key" == $'\033' ]]; then
                # Escape sequence (arrow keys)
                local seq
                seq=$(dd bs=1 count=2 2>/dev/null)
                case "$seq" in
                    "[A"|"OA")  # Up arrow
                        if [[ $selected -gt 0 ]]; then
                            ((selected--))
                            stty "$saved_stty"  # Restore briefly for output
                            draw_menu redraw
                            stty raw -echo
                        fi
                        ;;
                    "[B"|"OB")  # Down arrow
                        if [[ $selected -lt $((num_packages - 1)) ]]; then
                            ((selected++))
                            stty "$saved_stty"  # Restore briefly for output
                            draw_menu redraw
                            stty raw -echo
                        fi
                        ;;
                esac
            elif [[ "$key" == $'\003' ]]; then
                # Ctrl-C
                stty "$saved_stty"
                return 2
            fi
        done

        # Restore terminal state
        stty "$saved_stty"
        echo
        PKG_SOURCE_DIR=$(realpath "${package_paths[$selected]}")
        print_success "Selected: ${packages[$selected]}"
    fi

    return 0
}

tag_release() {
    print_step "Tagging release in your repository..."

    local tag="$PKG_VERSION"

    # Check if tag already exists
    if git tag -l | grep -q "^$tag\$"; then
        print_warning "Tag $tag already exists"
        if ! prompt_confirm "Delete and recreate it?" y; then
            return $?
        fi
        run_cmd "git tag -d $tag"
        run_cmd "git push origin --delete $tag 2>/dev/null"
    fi

    run_cmd "git tag -a $tag -m 'Release version $PKG_VERSION'"
    print_success "Created tag $tag"

    run_cmd "git push origin $tag"
    print_success "Pushed tag to origin"

    return 0
}

setup_packages_clone() {
    print_step "Setting up typst/packages clone..."

    if [[ -d "$PACKAGES_DIR" ]]; then
        print_info "Directory exists at $PACKAGES_DIR"
        prompt_confirm "Re-clone fresh?" n
        local status=$?
        case $status in
            0)
                # Yes - remove and re-clone
                run_cmd "rm -rf '$PACKAGES_DIR'"
                ;;
            1)
                # No - just sync with upstream
                local pkg_path="packages/preview/$PKG_NAME"
                print_info "Syncing with upstream..."
                if [[ "$DRY_RUN" == true ]]; then
                    print_dry "cd $PACKAGES_DIR"
                    print_dry "git sparse-checkout add $pkg_path"
                    print_dry "git fetch upstream"
                    print_dry "git checkout main"
                    print_dry "git merge upstream/main"
                else
                    cd "$PACKAGES_DIR"
                    # Ensure sparse-checkout includes this package
                    git sparse-checkout add "$pkg_path"
                    git fetch upstream
                    git checkout main
                    git merge upstream/main --no-edit 2>/dev/null || true
                fi
                print_success "Synced with upstream"
                return 0
                ;;
            2)
                # Ctrl-C
                return 2
                ;;
        esac
    fi

    print_info "Forking repository..."
    run_cmd "gh repo fork typst/packages --clone=false 2>/dev/null"

    print_info "Cloning with sparse checkout (this is fast)..."

    local pkg_path="packages/preview/$PKG_NAME"

    if [[ "$DRY_RUN" == true ]]; then
        local gh_user
        gh_user=$(gh api user -q .login 2>/dev/null || echo "YOUR-USERNAME")
        print_dry "git clone --sparse --filter=blob:none --depth=1 git@github.com:$gh_user/packages.git $PACKAGES_DIR"
        print_dry "cd $PACKAGES_DIR"
        print_dry "git remote add upstream https://github.com/typst/packages.git"
        print_dry "git sparse-checkout set $pkg_path"
        print_dry "git fetch upstream main"
        print_dry "git merge upstream/main"
    else
        local gh_user
        gh_user=$(gh api user -q .login)
        git clone --sparse --filter=blob:none --depth=1 "git@github.com:$gh_user/packages.git" "$PACKAGES_DIR" &
        local clone_pid=$!
        spinner $clone_pid
        wait $clone_pid
        local clone_status=$?

        if [[ $clone_status -ne 0 ]]; then
            # Fallback to HTTPS
            git clone --sparse --filter=blob:none --depth=1 "https://github.com/$gh_user/packages.git" "$PACKAGES_DIR"
        fi

        cd "$PACKAGES_DIR"
        git remote add upstream https://github.com/typst/packages.git 2>/dev/null || true

        # Configure sparse-checkout to include the package directory
        print_info "Configuring sparse checkout for $pkg_path..."
        git sparse-checkout set "$pkg_path"

        # Fetch and merge upstream to get existing versions
        print_info "Fetching upstream to check for existing versions..."
        git fetch upstream main
        git merge upstream/main --no-edit 2>/dev/null || true
    fi

    print_success "Packages repository ready"
    return 0
}

copy_package() {
    print_step "Copying package to packages repository..."

    local dest="$PACKAGES_DIR/packages/preview/$PKG_NAME/$PKG_VERSION"

    if [[ -d "$dest" ]]; then
        print_warning "Directory already exists: $dest"
        if ! prompt_confirm "Overwrite?" y; then
            return $?
        fi
        run_cmd "rm -rf '$dest'"
    fi

    run_cmd "mkdir -p '$dest'"

    # Copy all files from the source directory
    run_cmd "cp -r '$PKG_SOURCE_DIR'/. '$dest/'"
    print_success "Copied all files to $dest"

    return 0
}

push_changes() {
    print_step "Pushing changes..."

    if [[ "$DRY_RUN" == true ]]; then
        print_dry "cd $PACKAGES_DIR"
    else
        cd "$PACKAGES_DIR"
    fi

    local branch="$PKG_NAME-$PKG_VERSION"

    if [[ "$DRY_RUN" == true ]]; then
        run_cmd "git checkout -b $branch"
        run_cmd "git add ."
        run_cmd "git commit -m 'Add $PKG_NAME $PKG_VERSION'"
        run_cmd "git push -u origin $branch"
    else
        if git branch -a | grep -q "$branch"; then
            print_warning "Branch $branch already exists"
            git checkout "$branch"
            git add .
            git commit --amend --no-edit 2>/dev/null || true
            git push -f origin "$branch"
        else
            git checkout -b "$branch"
            git add .
            git commit -m "Add $PKG_NAME $PKG_VERSION"
            git push -u origin "$branch"
        fi
    fi

    print_success "Pushed branch $branch"

    return 0
}

show_summary() {
    print_header "Summary"

    echo "  ${C_DIM}Package:${C_RESET}     $PKG_NAME"
    echo "  ${C_DIM}Version:${C_RESET}     $PKG_VERSION"
    echo "  ${C_DIM}Description:${C_RESET} $PKG_DESCRIPTION"
    echo "  ${C_DIM}Source:${C_RESET}      $PKG_SOURCE_DIR"
    echo
}

# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

main() {
    # Parse arguments
    for arg in "$@"; do
        case "$arg" in
            -n|--dry-run)
                DRY_RUN=true
                ;;
            -h|--help)
                echo "Usage: typst-publish.sh [OPTIONS]"
                echo
                echo "Options:"
                echo "  -n, --dry-run    Walk through all steps without making changes"
                echo "  -h, --help       Show this help message"
                return 0
                ;;
        esac
    done

    if [[ "$DRY_RUN" == true ]]; then
        print_header "Typst Universe Publisher (DRY RUN)"
        echo "${C_YELLOW}  ⚠ Dry run mode: no changes will be made${C_RESET}"
        echo "${C_DIM}  Commands that would be executed are shown in yellow${C_RESET}"
        echo
    else
        print_header "Typst Universe Publisher"
    fi

    # Check prerequisites
    if ! check_prerequisites; then
        return $?
    fi
    echo

    # Select package from release/preview
    if ! select_package; then
        return $?
    fi
    echo

    # Parse typst.toml
    if ! parse_typst_toml "$PKG_SOURCE_DIR/typst.toml"; then
        return $?
    fi

    show_summary

    if ! prompt_confirm "Continue with this package?" y; then
        return $?
    fi
    echo

    local original_dir
    original_dir=$(pwd)

    # Step 1: Setup packages clone
    print_header "Step 1: Setup Packages Repository"

    if ! setup_packages_clone; then
        return $?
    fi
    echo

    # Step 2: Copy package
    print_header "Step 2: Copy Package"

    if ! copy_package; then
        return $?
    fi
    echo

    # Step 3: Push changes
    print_header "Step 3: Push Changes"

    if ! push_changes; then
        return $?
    fi
    echo

    # Step 4: Tag release (final step, after changes are pushed)
    print_header "Step 4: Tag Release"

    if [[ "$DRY_RUN" != true ]]; then
        cd "$original_dir"
    else
        print_dry "cd $original_dir"
    fi

    if git -C "$original_dir" rev-parse --git-dir >/dev/null 2>&1; then
        prompt_confirm "Tag $PKG_VERSION in your repository?" y
        local tag_status=$?
        case $tag_status in
            0)
                tag_release
                ;;
            2)
                return 2
                ;;
        esac
    else
        print_warning "Not a git repository, skipping tagging"
    fi
    echo

    # Done
    if [[ "$DRY_RUN" == true ]]; then
        print_header "Dry Run Complete"
        echo "  No changes were made. Run without --dry-run to execute."
    else
        print_header "Done!"
        echo "  Your changes have been pushed. Create a PR using the link above."
        echo "  The Typst team will review your PR and merge it."
    fi

    echo
    echo "  ${C_DIM}Once merged, users can import your package with:${C_RESET}"
    echo "  ${C_CYAN}#import \"@preview/$PKG_NAME:$PKG_VERSION\"${C_RESET}"
    echo
}

# Run if executed directly
main "$@"
