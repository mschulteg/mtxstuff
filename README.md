# mtxstuff
A tool to manage the subtitle and audio track flags of mkv files by using mkvpropedit.

This program is undocumented and unfinished. Use at your own risk.
Only linux is currently supported.

# how it works
In TUI mode Press 'S' to access Subtitle view, 'A' to access audio track view.
Files are scanned and put into groups that share the same track metadata (name, lang, flags).
This makes it easy to change metadata on multiple files that share the same general track list shape.
Changes are applied to all files in a group!

# usage
Editing can be done using CLI args or using the TUI.
The TUI is probably the more stable feature, to open it use:

```bash
# TUI
mtxstuff tui /dir/with/mkvfiles
```
# runtime dependencies
- mkvpropedit and mkvmerge need to be availabie in the PATH