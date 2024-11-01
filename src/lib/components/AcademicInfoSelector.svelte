<script>
    import {invoke} from "@tauri-apps/api/core";
    import {createEventDispatcher} from "svelte";

    // create a dispatcher
    const dispatch = createEventDispatcher();

    // event to submit the selected values
    function handleSubmit() {
        dispatch("submit", {
            termIds: terms.filter(t => t.isSelected).map(t => t.termId),
            majorId: parseInt(selectedMajor),
            grade: selectedGrade.toString().slice(2, 4),
            classId: selectedClass === '-1' ? undefined : parseInt(selectedClass),
        });
    }


    // the selected values
    let terms = [];
    let selectedClass = "-1";
    let selectedGrade = "-1";
    let selectedMajor = "-1";
    let selectedCollege = "-1";

    // toggle term selection
    function toggle(term) {
        term.isSelected = !term.isSelected;
        terms = [...terms];
    }

    // get terms
    async function getTerms() {
        const fetchedTerms = await invoke("get_terms");
        fetchedTerms.sort((a, b) => a.termName.localeCompare(b.termName));
        terms = fetchedTerms.map(term =>
            ({...term, isSelected: false}));
        console.log(terms);
    }

</script>


<div>
    <!-- term selector -->
    <div class="pb-4 space-x-1 flex flex-wrap">
        <span class="bg-clip-text">学期:</span>
        {#await getTerms()}
            <p>loading...</p>
        {:then _}
            {#each terms as t}
                <button
                        class="chip {t.isSelected ? 'variant-filled' : 'variant-soft'}"
                        on:click={() => { toggle(t); }}
                >
                    <span class="capitalize">{t.termName}</span>
                </button>
            {/each}
        {:catch error}
            <p>{error}</p>
        {/await}
    </div>

    <div class="flex pb-3 space-x-1">
        <!-- select college -->
        <select class="select" bind:value={selectedCollege}>
            <option value="-1" selected>学院</option>
            {#await invoke("get_colleges") then colleges}
                {#each colleges as c}
                    <option value={c.collegeId}>{c.collegeName}</option>
                {/each}
            {/await}
        </select>

        <!-- select major -->
        <select class="select" bind:value={selectedMajor}>
            <option value="-1" selected>专业</option>
            {#await invoke("get_majors", {collegeId: parseInt(selectedCollege)}) then majors}
                {#each majors as m}
                    <option value={m.majorId}>{m.majorName}</option>
                {/each}
            {/await}
        </select>

        <!-- select grade -->
        <select class="select" bind:value={selectedGrade}>
            <option value="-1" selected>年级</option>
            {#each (() => {
                let grades = terms.map((term) => term.termName.split("-")[0]).sort();
                grades = [...new Set(grades)];
                return Array.from({length: grades.length + 3}, (_, i) => grades[0] - 3 + i);
            })()
                    as t}
                <option value={t}>{t}</option>
            {/each}
        </select>

        <!-- select class -->
        <select class="select" bind:value={selectedClass}>
            <option value="-1" selected>班级</option>
            {#await invoke("get_classes", {majorId: parseInt(selectedMajor)}) then classes}
                {#each classes as c}
                    <option value={c.classId}>{c.className}</option>
                {/each}
            {/await}
        </select>

        <!-- submit button -->
        <button type="button" class="btn btn-md variant-filled"
                disabled={ terms.length === 0 || selectedCollege === '-1' || selectedMajor === '-1' || selectedGrade === '-1'}
                on:click={handleSubmit}>查询
        </button>
    </div>

</div>
