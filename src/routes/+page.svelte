<script>
    import { invoke } from "@tauri-apps/api/core";
    import Datatable from "$lib/components/table/Datatable.svelte";
    import AcademicInfoSelector from "$lib/components/AcademicInfoSelector.svelte";
    import DataImporter from "$lib/components/DataImporter.svelte";
    import { tableData } from "../store.js";

    function handleAcademicInfoSubmit(event) {
        let data = event.detail;
        console.log({
            terms: data.termIds,
            majorId: data.majorId,
            grade: data.grade,
            classId: data.classId,
        });
        invoke("get_gpa", {
            terms: data.termIds,
            majorId: data.majorId,
            grade: data.grade,
            classId: data.classId,
        })
            .then((response) => {
                let sortedData = response.sort((a, b) => b.gpa - a.gpa);
                sortedData.forEach((item, index) => {
                    item.ord = index + 1;
                });
                tableData.set(sortedData);
                console.log($tableData);
            })
            .catch((error) => {
                console.error(error);
            });
    }
</script>

<svelte:head>
    <title>Home</title>
    <meta name="description" content="Home Page" />
</svelte:head>

<section>
    {#await invoke("get_terms") then terms}
        {#if terms.length === 0}
            <DataImporter />
        {:else}
            <AcademicInfoSelector on:submit={handleAcademicInfoSubmit} />
            <Datatable />
        {/if}
    {/await}
</section>
