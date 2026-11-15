//! A bash function completing `cargo xtask` and its aliases.

/// The bash function `completions` prints, registered for
/// `cargo-xtask`, the name cargo's completion hands `cargo xtask` to,
/// and for each of `names`.
///
/// It is clap's own registration script with one change: where that
/// calls one binary by name, this runs `cargo xtask`, so the answer is
/// the xtask's of the repo the shell is in. A repo with no xtask, or
/// one that fails, completes nothing.
pub fn script(names: &[String]) -> String {
    let mut registered = vec!["cargo-xtask".to_string()];
    registered.extend(names.iter().cloned());
    format!(
        r#"# Completion for `cargo xtask`, printed by `cargo xtask completions`.
# It asks the xtask of the repo the shell is in, on each tab.
_cargo_xtask_complete() {{
    local IFS=$'\013'
    local _CLAP_COMPLETE_INDEX=${{COMP_CWORD}}
    local _CLAP_COMPLETE_COMP_TYPE=${{COMP_TYPE}}
    if compopt +o nospace 2> /dev/null; then
        local _CLAP_COMPLETE_SPACE=false
    else
        local _CLAP_COMPLETE_SPACE=true
    fi
    local words=("${{COMP_WORDS[@]}}")
    if [[ "${{BASH_VERSINFO[0]}}" -ge 4 ]]; then
        words[COMP_CWORD]="$2"
    fi
    COMPREPLY=( $( \
        _CLAP_IFS="$IFS" \
        _CLAP_COMPLETE_INDEX="$_CLAP_COMPLETE_INDEX" \
        _CLAP_COMPLETE_COMP_TYPE="$_CLAP_COMPLETE_COMP_TYPE" \
        _CLAP_COMPLETE_SPACE="$_CLAP_COMPLETE_SPACE" \
        COMPLETE="bash" \
        cargo xtask -- "${{words[@]}}" 2> /dev/null \
    ) )
    if [[ $? != 0 ]]; then
        unset COMPREPLY
    elif [[ $_CLAP_COMPLETE_SPACE == false ]] && [[ "${{COMPREPLY-}}" =~ [=/:]$ ]]; then
        compopt -o nospace
    fi
}}
if [[ "${{BASH_VERSINFO[0]}}" -eq 4 && "${{BASH_VERSINFO[1]}}" -ge 4 || "${{BASH_VERSINFO[0]}}" -gt 4 ]]; then
    complete -o nospace -o bashdefault -o nosort -F _cargo_xtask_complete {names}
else
    complete -o nospace -o bashdefault -F _cargo_xtask_complete {names}
fi
"#,
        names = registered.join(" ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The function asks `cargo xtask`, not a binary by name, and is
    /// registered for cargo's hand-off name and each alias given.
    #[test]
    fn the_completion_function_asks_the_repos_xtask() {
        let script = script(&["cx".to_string()]);
        assert!(
            script.contains("cargo xtask -- \"${words[@]}\" 2> /dev/null"),
            "{script}"
        );
        // Registered twice over, as clap's script is: unsorted where
        // bash can, 4.4 and later, so the tasks keep their order.
        assert!(
            script.contains("-o nosort -F _cargo_xtask_complete cargo-xtask cx\n"),
            "{script}"
        );
        assert!(
            script.ends_with("-F _cargo_xtask_complete cargo-xtask cx\nfi\n"),
            "{script}"
        );
        assert!(super::script(&[]).ends_with("_cargo_xtask_complete cargo-xtask\nfi\n"));
    }

    /// The script is clap's text as written: the IFS escape is the four
    /// characters `\013` for bash to read, never the vertical tab itself,
    /// and the completer's environment is one assignment per line joined
    /// by `\` continuations, no run of spaces where a join was lost.
    #[test]
    fn the_completion_script_is_claps_text() {
        let script = script(&[]);
        assert!(script.contains("local IFS=$'\\013'\n"), "{script}");
        assert!(!script.contains('\x0b'), "a raw vertical tab: {script}");
        assert!(
            script.contains("COMPREPLY=( $( \\\n        _CLAP_IFS=\"$IFS\" \\\n"),
            "{script}"
        );
        assert!(script.contains("2> /dev/null \\\n    ) )\n"), "{script}");
        let joined = script.lines().find(|l| l.trim_start().contains("   "));
        assert_eq!(joined, None, "a lost join leaves a run of spaces mid-line");
    }
}
