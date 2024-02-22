# Yggit

Hello world

A tool to manage my git workflow.

It allows me to have one branch, and to associate a commits to a specific branch with a interface like the rebase one

# How I am using it?

First I use git to have a beautiful history. To do so I am using `git-rebase`.

Then when I am ready to push my commits in different branch I just have to use `yggit push`.

A _rebase like_ interface will open with the editor specified in your git configuration.

> Do not edit/move your commit in this editor, it won't have any effects.

Then if I want to push a specific commit on a specific branch I just have to write under the given commit:

```bash
-> mybranch-name
```

I can also specify a custom upstream:

```bash
-> origin:mybranch-name
```

# Warning

This project is poorly tested, use it at your own risk.

# Acknowledgements

This project was greatly inspired by [anger](https://github.com/d4hines/anger) by [d4hines](https://github.com/d4hines).
