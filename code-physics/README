This small set of scripts outlines how to start VSCode tunnels on the compute nodes of the Phsyics cluster, which you can connect to from a local VSCode instance (or even the browser) to get a complete IDE (editing, debugging, running calculations, using extensions) on the cluster without being smited for running heavy processes on headnode.

## Setup
In a shell on headnode, clone this repo and copy `/physics-cluster/code-physics/` to `~/code-physics/`.
Navigate to some build directory, e.g. `~/build/`, then install:
1. Vscode cli: `wget -O codecli https://code.visualstudio.com/sha/download?build=stable&os=cli-alpine-x64`
(`tar -xf` the downloaded file and find the `code` binary inside)
2. Miniforge, to get conda: follow https://github.com/conda-forge/miniforge
(or another conda supplier if you prefer. Make sure to `conda init` for both bash and tcsh)
3. Tmux via conda: `conda install conda-forge::tmux`
(you can install other helpful things too, like the fish shell or the 'killall' command, through conda)
4. Julia via juliaup: https://github.com/JuliaLang/juliaup

Add the relevant binaries to your path (in both `~/.tcshrc` and `~/.bashrc`; see examples in this repo).

### Starting a code tunnel
On headnode:
```
tmux # Sets up a persistent shell on headnode (won't terminate on headnode when you close the local terminal window)
qsub -I -l mem=32GB -l ncpus=16 -l walltime=24:00:00
```
This starts a persistent interactive job and gives you a compute node shell (the first flag is dash-capital-eye, the others are dash-el). You can change the `ncpus`, `mem`, and `walltime` requirements as you like.
Then in the interactive job shell, run:
```
code tunnel
```
Which will start a vscode server that you can connect to from a local vscode instance (install the remote-tunnels extension locally, sign in with github, and use the vscode command `connect to tunnel`), or even from the browser (following the link shown in the terminal output after authenticating).
Both should behave exactly like a local vscode session, just that Julia, any extensions, and other processes started from the terminal will be run on the compute node.

The first time you start `code tunnel`, you will need to accept the license terms and authenticate via github (following the prompts in the shell).
To cache the authentication, set `VSCODE_CLI_USE_FILE_KEYCHAIN=1` as an environment variable in both your `~/.bashrc` and `~/.tcshrc`.
Also note that you can only have one tunnel running on the cluster at a time, but multiple code sessions can connect to the same tunnel.

### Automation
The above steps for opening a vscode tunnel are easier to debug, but you aren't advised to have long-running interactive jobs on the cluster. Once things work following the steps above, you can use the PBS script [`code-physics.pbs`](https://github.com/brendanjohnharris/physics-cluster/tree/main/code-physics/code-physics.pbs) to submit a regular job that initializes the code tunnel and remains running for 24 hours (you should `qdel` the job when you are finished your calculations, though, or set a smaller `ncpus` and `mem` for editing and debugging purposes). To monitor this job, `tail -f ~/code-physics/code-physics.log`.

