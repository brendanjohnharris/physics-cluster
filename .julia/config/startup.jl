ENV["JULIA_CPU_TARGET"] = "generic; native"
ENV["JULIA_PKG_USE_CLI_GIT"] = true
ENV["LD_LIBRARY_PATH"] = ""
ENV["PATH"] = "/headnode2/bhar9988/.conda/envs/bhar9988/bin:$(ENV["PATH"])"
# ENV["JULIA_CONDAPKG_OFFLINE"] = "yes"
using Pkg
# ENV["PYTHON"]="/headnode2/bhar9988/.conda/envs/bhar9988/bin/python"
# ENV["JULIA_PYTHONCALL_EXE"]="@PyCall"
using Revise
using OhMyREPL
using Downloads
colorscheme!("OneDark")
enable_autocomplete_brackets(false)
function template()
    @eval begin
        using PkgTemplates
        stylefile = tempname()
        Downloads.download("https://gist.githubusercontent.com/brendanjohnharris/182f2deec122d16d28218d39ebecc9c8/raw/744b9950af52a949cb9bd4c6df4351d049cc37ca/.JuliaFormatter.toml", stylefile)
        Template(; user="brendanjohnharris",
            dir="./",
            julia=v"1.6.0",
            plugins=[ProjectFile(),
                SrcDir(),
                Tests(; project=true),
                Readme(),
                License(),
                Git(; ignore=["*.code-workspace", "*.mat", "*.csv", "*.parquet", "*.jld2", "data", "*.jl.cov", "*.jl.*.cov", "*.jl.mem", "docs/build/", "docs/site/", "LocalPreferences.toml", ".CondaPkg/", "Artifacts.toml", "Manifest.toml", ".vscode"]),
                CompatHelper(),
                TagBot(),
                GitHubActions(; linux=true, osx=true, windows=true, x86=true, extra_versions=["1.6", "nightly"]),
                Codecov(),
                Documenter{GitHubActions}(),
                Dependabot(),
                RegisterAction(),
                Formatter(; file=stylefile)],)
    end
end
