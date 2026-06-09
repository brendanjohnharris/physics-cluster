# AI instructions

## Global instructions
Please use em dashes (--- or unicode equivalent) sparingly; interjections are ok, but not so much if they close a sentence, where you shoudl prefer semicolons or colons (--- should typically be paired). NEVER put spaces around em dashes---always like this. Always be gramatically correct though.

## Coding

### Documentation and docstrings

Please use a slightly terse style for documentation, focusing on clarity and precision rather than hyping hte software.

### Julia language


#### Code evaluation

If you are considering running julia code in the terminal, especially if you are planning to write a new small script just to evaluate a small piece of code, please check if the Kaimon mcp is available, and if there is a connected repl for the current project. If so, please run code snippets there instead of the terminal.

#### Plotting

Please use CairoMakie as the plotting backend for all Julia code.
If the project you are working in has 'Fathom.jl' installed, please use 'Fathom.jl' for theming (it also has a number of recipes you should prefer over the CairoMakie defaults; e.g. prefer 'ziggurat' plots to 'hist' plots). This means `using CairoMakie; using Fathom; set_theme!(fathom())`. This sets up the defualt color order to follow the Fathom colors automatically, so for simple plots ther eis no need to set colors manually. For more complicated plots, please draw colors based on the guide below, and consult the `Fathom.colororder`.

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

Prefer to explicitly 'import' symbols from packages that are used sparsely; if many methods are used form a package (e.g. CairoMakie in a plot script, or MoreMaps in a calculation script), then it is fine to 'using' the package instead.

#### Mapping/iterating

Please install an duse the 'MoreMaps.jl' package for mapping and iterating over array-like collections. It provides a more consistent and powerful interface than the built-in 'map' and 'broadcast' functions. In general, consider if a for loop over an array can be re-written using 'map' syntax, and, if the map is substantial, use MoreMaps to do so.
In general:
- if the map is small and should run in <1 second, use built-in map
- if the map has many iterations and would take more than a few seconds, prefer to use the Threaded() backend with a progress logger such as 'ProgressLogger()` (you could also consider QualityLogger() or LogLogger()).


## For academic writing (use these guidelines when writing academic manuscripts; not for general purpose text or code)

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