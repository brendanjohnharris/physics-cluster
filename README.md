# Configuration for the USyd School of Physics computing cluster

See also [clustertools](https://github.com/brendanjohnharris/clustertools) for more tips on cluster computing.

## VSCode setup
One effective way of interacting with the physics cluster is via VSCode remote SSH tunnels. This setup will give you a VSCode instance that runs on a compute node, but is as interactive as a local VSCode instance.

### 1. Install VSCode locally
Instructions [here](https://code.visualstudio.com/download).
Also install the Remote Tunnels extension

### 2. Connect to cluster via ssh
Open a terminal in local VSCode. ssh to the physics cluster:
```

```

### 3. Install remote server on the cluster
In a terminal on the cluster, create a build directory:
```
mkdir $HOME/build
```
Install the VSCode CLI:
```
cd $HOME/build
curl -Lk 'https://code.visualstudio.com/sha/download?build=stable&os=cli-alpine-x64' --output vscode_cli.tar.gz
tar -xf vscode_cli.tar.gz
```
Then add code cli binary folder to your PATH

### 4. Start a remote VSCode job
...code-physics script...
