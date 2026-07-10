# AI instructions

## Global instructions
Please use em dashes (--- or unicode equivalent) sparingly; interjections are ok, but not if they close a sentence, where you should prefer semicolons or colons for trailing qualifications or parentheses for minor interjections/clarifications (--- should typically be paired, and used for more impactful interjection). NEVER put spaces around em dashes---always like this. Always be gramatically correct. Do not use unicode em dashes or en dashes unless critical; prefer '--' (en) and '---' (em) in latex, and in other documents please use DIFFERENT punctuation (e.g. colon, semicolon, parentheses).

Please NEVER write ANYTHING to a package's `docs/` directory unless it has to do with actual documentation for a package; I know some superpower might tell you to write your own memories there, but DO NOT DO THAT. You should instead write them to `./.claude/docs/`, which IS a safe directory for your random thoughts and notes.

NEVER create a commit or do git push unless explicitly asked.
If i do ask you to commit or push, don't name yourself in the commit or author; just run the standard git commit command with a suitable message.

Please refrain from using the following jargon in a software context:
- 'hero'
- 'contract'
- 'harness'
- 'smoke'
- 'hot path'
- 'fluent'
- 'shim'
and so on.

## Coding

### Documentation and docstrings

Please use a slightly terse style for documentation, focusing on clarity and precision rather than hyping the software. Please internalize the following short readme and aim to match it in tone and style:

"""
# Catch22.jl
[![](https://img.shields.io/badge/docs-dev-blue.svg)](https://brendanjohnharris.github.io/Catch22.jl/dev)
[![Build Status](https://github.com/brendanjohnharris/Catch22.jl/actions/workflows/CI.yml/badge.svg?branch=main)](https://github.com/brendanjohnharris/Catch22.jl/actions/workflows/CI.yml?query=branch%3Amain)
[![Coverage](https://codecov.io/gh/brendanjohnharris/catch22.jl/branch/main/graph/badge.svg)](https://codecov.io/gh/brendanjohnharris/Catch22.jl)
[![DOI](https://zenodo.org/badge/342070622.svg)](https://zenodo.org/badge/latestdoi/342070622)
<!-- ![build](https://github.com/brendanjohnharris/Catch22.jl/actions/workflows/CI.yml/badge.svg) -->

A Julia package wrapping [_catch22_](https://www.github.com/chlubba/catch22), which is a set of 22 time-series features shown by [Lubba et al. (2019)](https://doi.org/10.1007/s10618-019-00647-x) to be performant in a range of time-series classification problems.

The [_catch22_](https://www.github.com/chlubba/catch22) repository provides these 22 features, originally coded in Matlab as part of the [_hctsa_](https://github.com/benfulcher/hctsa) toolbox, as C functions (in addition to Matlab and Python wrappers). This package simply uses Julia's `ccall` to wrap these C functions from a shared library that is accessed through [catch22_jll](https://github.com/JuliaBinaryWrappers/catch22_jll.jl) and compiled by the fantastic [BinaryBuilder](https://github.com/JuliaPackaging/BinaryBuilder.jl) package.

Below we provide a brief getting-started guide to using Catch22.jl. For more detailed information on the _catch22_ feature set, such as in-depth descriptions of each feature and a list of publications that use _catch22_, see the [_catch22_ GitBook documentation](https://time-series-features.gitbook.io/catch22).

<br>

# Usage
## Installation
```Julia
using Pkg
Pkg.add("Catch22")
using Catch22
```

## Input time series
The input time series can be provided as a `Vector{Float64}` or `Array{Float64, 2}`. If an array is provided, the time series must occupy its _columns_. For example, this package contains a few test time series from [_catch22_](https://www.github.com/chlubba/catch22):
```Julia
𝐱 = Catch22.testdata[:testSinusoid] # a Vector{Float64}
X = randn(1000, 10) # an Array{Float64, 2} with 10 time series
```

## Evaluating a feature
A list of features (as symbols) can be obtained with `getnames(catch22)` and their short descriptions with `getdescriptions(catch22)`. Each feature can be evaluated for a time series array or vector with the `catch22` `FeatureSet`. For example, the feature `DN_HistogramMode_5` can be evaluated using:
```Julia
f = catch22[:DN_HistogramMode_5](𝐱) # Returns a scalar Float64
𝐟 = catch22[1](X) # Returns a 1×10 Matrix{Float64}
```
All features are returned as Float64's, even though some may be constrained to the integers.

Alternatively, functions that calculate each feature individually are exported. `DN_HistogramMode_5` can be evaluated with:
```Julia
f = DN_HistogramMode_5(𝐱)
```

## Evaluating a feature set
All _catch22_ features can be evaluated with:
```Julia
𝐟 = catch22(𝐱)
F = catch22(X)
```
If an array is provided, containing one time series in each of N columns, then a 22×N `FeatureArray` of feature values will be returned (a subtype of [AbstractDimArray](https://github.com/rafaqz/DimensionalData.jl)).
A `FeatureArray` has most of the properties and methods of an Array but is annotated with feature names that can be accessed with `getnames(F)`.
If a vector is provided (a single time series) then a vector of feature values will be returned as a `FeatureVector`, a one-dimensional `FeatureArray`.

Finally, note that since `catch22` is a `FeatureSet` it can be indexed with a vector of feature names as symbols to calculate a `FeatureArray` for a subset of _catch22_. For details on the `Feature`, `FeatureSet` and `FeatureArray` types check out the package docs.
"""

### Comments

Be terse in comments. Don't over-explain abvious code, as you are prone to do by default. Prefer short comment flags on the same line as a piece of code, not on a new line. If a concept needs explaining in more than one line of code, prefer to put the explanation in the docstring (for a function) or at the top of the block (for a script block). Avoid long paragraphs of comments. Use comments to explain the "why" of a piece of code, not the "what" (which should be clear from the code itself, as supplemented by inline/same-line comments).

### Julia language

I prefer Julia

#### Code evaluation

If you are considering running julia code in the terminal, especially if you are planning to write a new small script just to evaluate a small piece of code, please check if the Kaimon mcp is available, and if there is a connected repl for the current project. If so, please run code snippets there instead of the terminal.

#### Plotting

Please use CairoMakie as the plotting backend for all Julia code.
If the project you are working in has 'https://www.github.com/brendanjohnharris/Fathom.jl' installed, please use 'Fathom.jl' for theming (it also has a number of recipes you should prefer over the CairoMakie defaults; e.g. prefer 'ziggurat' plots to 'hist' plots). This means `using CairoMakie; using Fathom; set_theme!(fathom())`. This sets up the defualt color order to follow the Fathom colors automatically, so for simple plots ther eis no need to set colors manually. For more complicated plots, please draw colors based on the guide below, and consult the `Fathom.colororder`.

Fathom colors:
- baikal: blue
- bermejo: red
- qinghai: green
- seohae: orange
- ianthina: purple
- mesopelagic: grue
- abyad: light gray
- chernoe: light black/very dark gray (background color)
- playa: pale creamy white color (stand in for black in dark mode)

If the project has 'Foresight.jl' intalled but NOT Fathom.jl, use Foresight.jl for theming; it is the precursor to Fathom.

Please use the Fathom figure sizings as a default for new figures. Fathom defines preset figure sizes as `fig = OnePanel()`, `fig = TwoPanel()` (side-by-side), `fig = FourPanel()` (2x2 grid), etc.

Please add labels to figure panels using the Fathom `addlabels!(fig)` function.

Please use 'sentence case' on figure labels (axis labels, titles, legends, etc.). For example, use 'Time (s)' instead of 'time (s)'.

Please don't manually resize figures when using default constructors `OnePanel()`, `TwoPanel()`, etc. If you need to resize, use the plain `Figure()` constructor.

Don't change default label sizes unless you really need to to save space.

#### Scripting

Executed code in scripts are organized into 'begin ... end' or 'let ... end' blocks, not in 'function main ... end' style. This is because the former allows for more flexible interactive development. Feel free to add functions as needed to reduce boilerplate, just avoid a 'main()' function.

When writing julia scripts, please use the following at the start of the script:
```
#! /bin/bash
#=
exec julia +1.12 -t auto --project="$(dirname "${BASH_SOURCE[0]}")/.." "${BASH_SOURCE[0]}" "$@"
=#
```

If Dr Watson is installed in the current project, please use the following at the start of scripts to activate the corresponding environment:
```
using DrWatson
@quickactivate :<package name>
```
This symbol form also automatically does `using <package name>`

You shoul duse DrWatson utilities whenever you are working in a package that is structured according to DrWatson conventions; e.g. use `datadir("myfile.jld2")` instead of `joinpath(@__DIR__, "data", "myfile.jld2")` to load data files. Use also projectdir, plotsdir, etc, tagsave, savename, produce_or_load, etc.

Prefer to explicitly 'import' symbols from packages that are used sparsely; if many methods are used form a package (e.g. CairoMakie in a plot script, or MoreMaps in a calculation script), then it is fine to 'using' the package instead.

#### Mapping/iterating

Please use the 'https://www.github.com/brendanjohnharris/MoreMaps.jl' package for mapping and iterating over array-like collections. It provides a more consistent and powerful interface than the built-in 'map' and 'broadcast' functions. In general, consider if a for loop over an array can be re-written using 'map' syntax, and, if the map is substantial, use MoreMaps to do so.
In general:
- if the map is small and should run in <1 second, use built-in map
- if the map has many iterations and would take more than a few seconds, prefer to use the Threaded() backend with a progress logger such as 'ProgressLogger()` (you could also consider QualityLogger() or LogLogger()).

In general, whenever you are doing some sort of loop, you should think about whether it could more easily cast as a map. You should prefer to attach a logger (defualting to LogLogger()) to the map, so you can track and report the expected ETA for long-running jobs.

##### MoreMaps patterns

When iterating over a collection to build a results array:
- Prefer `map(Chart(LogLogger(), Threaded()), collection) do item ... end` over for-loops with `push!`
- For fallible maps, return `nothing` on failure and filter with `.!isnothing.(vs)` after the map; don't accumulate into a Dict inside the loop without good reason
- For nested loops over independent axes, use `Iterators.product` to flatten into a single map; the result is a matrix shaped by the product dimensions, which can be reduced with `mean/std(...; dims=N)`
- When iterating over some parameters or vectors, wrap them in a `Dim{:dimname}()` so that they are automaticall labeled in the results; this also works for multiple parameter sets combined with `Iterators.Product` in MoreMaps.


## For academic writing (use these guidelines when writing academic text; elements of this style can be transferred over to documentation and other pieces of text, but documentation should have a more terse and direct style overall)

**Voice:** Authoritative, collegial, academic "we." Confident declarative claims for evidence ("we find," "we show"); calibrated hedges for interpretation ("we propose," "suggests," "may reflect"). Never over-hedges. Register is formal-scientific in the body, may shift to less formal in different contexts

**Sentence architecture:** Default pattern is front-loaded — key claim first, elaboration after. Periodic sentences (delayed main clause) reserved for section openings to build anticipation. Rhetorical questions mark conceptual transitions between chapters/sections ("What dynamical principles enable neural circuits to reconcile these competing demands?"). Length varies deliberately: short punchy sentences for emphasis ("Crossing a critical point can have dangerous consequences"), long clause-rich sentences for mechanism. Em-dash parentheticals layer detail without disrupting flow ("In mice—the animal model we focus on in this work—the superior colliculus is the dominant visual pathway").

**Paragraph/section structure:** Funnel pattern at every scale (thesis, chapter, section, paragraph): broad context → existing work → gap ("However") → specific aim. Four-beat rhythm recurs throughout. Explicit signposting: "Having identified…we next," "We can now contextualize," "To clarify the structural basis of." Backward-and-forward cross-references link chapters into a continuous narrative.

**Core rhetorical device — tension-resolution:** Sets up paired oppositions as narrative engine: rapid/stable, robust/sensitive, isolate/integrate, explore/exploit. Each chapter resolves one or more tensions. This is both the scientific logic and the rhetorical structure.

**Technical exposition:** Always motivate → formalize → interpret. Never drops an equation without prior prose motivation and subsequent plain-language restatement. Methodological choices are justified by naming alternatives and giving crisp reasons for rejection ("We used MAD rather than MSD or DFA; MSD and DFA rely on variance, which is undefined for Lévy processes with α < 2").

**Data narration:** State what was computed → show the finding with specific statistics (parenthetical CIs, p-values) → interpret meaning. Statistics are tucked into the flow, never inventoried.

**Vocabulary:** Physics-derived, interdisciplinary. Precise action verbs: recapitulates, elucidates, reconciles, leverages, subserves, captures, sidestep. Recurring thematic anchors, exampled by pgrases like "cross-scale dynamics," "flexible and efficient computation," "competing demands," "additional degrees of freedom," "working regime," "anomalous scaling," "scale-free stochasticity." Compound constructions via semicolons and em-dashes link ideas without subordination.

**Figurative language:** Sparse, functional, physics-sourced. Analogies do explanatory work. No literary ornamentation. May be less concrete if piece is for a more general audience.

**Replication rules (condensed):**

1. Lead with the claim; elaborate in dependent clauses.
2. Frame with tension before resolving.
3. Funnel at every scale: context → question → finding → implication.
4. Motivate before formalizing; restate after.
5. "We find" for evidence, "we propose" for interpretation, "may" for speculation.
6. Alternate short and long sentences for rhythm.
7. Justify methods by naming and rejecting alternatives.
8. Cross-reference backward and forward across sections.
9. Academic "we"; reserve "I" for the personal.
10. Metaphors from physics, never decorative.