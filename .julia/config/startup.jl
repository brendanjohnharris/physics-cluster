ENV["JULIA_CPU_TARGET"] = "generic"
ENV["JULIA_PKG_USE_CLI_GIT"] = true
ENV["LD_LIBRARY_PATH"] = ""
ENV["PATH"] = "/headnode2/bhar9988/.conda/envs/bhar9988/bin:$(ENV["PATH"])"
# ENV["JULIA_CONDAPKG_OFFLINE"] = "yes"
using Pkg
ENV["PYTHON"] = "/headnode2/bhar9988/.conda/envs/bhar9988/bin/python"
# ENV["JULIA_PYTHONCALL_EXE"] = "@PyCall"
ENV["FREETYPE_ABSTRACTION_FONT_PATH"] = "/headnode2/bhar9988/.conda/envs/bhar9988/fonts/"
ENV["JULIA_DEBUG"] = "SpatiotemporalMotifs" # loading,VSCodeServer
ENV["JULIA_DISTRIBUTED"] = true
# ENV["FORESIGHT_PATCHES"] = true
using Revise
using OhMyREPL
using Downloads
using Infiltrator
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


if !contains(gethostname(), "headnode") && haskey(ENV, "MOST_RECENT_SOCKET")
    sock = ENV["MOST_RECENT_SOCKET"]

    # cmd = Pipeline(`/bin/bash`, `ssh -nNT -L $sock:$sock headnode \&`)
    # if !issocket(sock)
    #     run(cmd)
    # end
    # using Sockets
    # using Dates

    # pushfirst!(LOAD_PATH, raw"/headnode2/bhar9988/.vscode-server/extensions/julialang.language-julia-1.72.0/scripts/packages")
    # try
    #     using VSCodeServer
    # finally
    #     popfirst!(LOAD_PATH)
    # end
    # VSCodeServer.serve(sock; is_dev="DEBUG_MODE=true" in Base.ARGS, crashreporting_pipename=raw"/tmp/vsc-jl-cr")
    # nothing # re-establishing connection with VSCode
end
if contains(gethostname(), "gpu")
    using CUDA
    CUDA.set_runtime_version!(v"12.5.0")
end
