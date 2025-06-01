# Yggit

A tool to manage my git workflow.

It allows me to have one branch, and to associate a commits to a specific branch with a interface like the rebase one

# How I am using it? What is my git workflow ?

First I am using `git` to have a beautiful history. To do so I am using `git-rebase`. My goal is to have a linear history and only incremental commits. IMO it's easier to review, and easier to manage when coding. By doing this exercice I've found out I am thinking a bit more before implemententing a solution

Then I will want to split my history in many branches, because a bug fix or a feature can be done in many steps, so in many branches.

To do so I just have to run `yggit push`

A _rebase like_ interface will open with the editor specified in your `git` configuration.

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

Even if I use this project daily in my work day. It is poorly tested, use it at your own risk.

# Acknowledgements

This project was greatly inspired by [anger](https://github.com/d4hines/anger) by [d4hines](https://github.com/d4hines).
