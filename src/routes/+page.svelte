<script>
    import {invoke} from "@tauri-apps/api/core";
    import Datatable from '$lib/components/Datatable.svelte';
    import AcademicInfoSelector from "$lib/components/AcademicInfoSelector.svelte";
    import {tableData} from "../store.js";


    function handleAcademicInfoSubmit(event) {
        let data = event.detail;
        console.log({
            terms: data.termIds,
            majorId: data.majorId,
            grade: data.grade,
            classId: data.classId
        });
        invoke("get_gpa", {
            terms: data.termIds,
            majorId: data.majorId,
            grade: data.grade,
            classId: data.classId
        })
            .then(response => {
                let sortedData = response.sort((a, b) => b.gpa - a.gpa);
                sortedData.forEach((item, index) => {
                    item.ord = index + 1;
                });
                tableData.set(sortedData);
                console.log($tableData);
            })
            .catch(error => {
                console.error(error);
            });
    }

</script>

<svelte:head>
    <title>Home</title>
    <meta name="description" content="Home Page"/>
</svelte:head>

<section>
    <AcademicInfoSelector on:submit={handleAcademicInfoSubmit}/>
    <Datatable/>
    <button on:click={() => console.log($tableData)}>Log</button>
</section>
