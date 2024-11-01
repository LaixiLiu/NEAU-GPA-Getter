<script>
    //Import local datatable components
    import ThSort from '$lib/components/ThSort.svelte';
    import ThFilter from '$lib/components/ThFilter.svelte';
    import Search from '$lib/components/Search.svelte';
    import RowsPerPage from '$lib/components/RowsPerPage.svelte';
    import RowCount from '$lib/components/RowCount.svelte';
    import Pagination from '$lib/components/Pagination.svelte';

    //Load local data
    // import data from '$lib/data/data.js';

    import {tableData} from "../../store.js";

    $: data = $tableData;

    //Import handler from SSD
    import {DataHandler} from '@vincjo/datatables';

    //Init data handler - CLIENT
    $: handler = new DataHandler(data, {rowsPerPage: 10});
    $: rows = handler.getRows();
</script>

<div class=" overflow-x-auto space-y-4">
    <!-- Header -->
    <header class="flex justify-between gap-4">
        <Search {handler}/>
        <RowsPerPage {handler}/>
    </header>
    <!-- Table -->
    <table class="table table-hover table-compact w-full table-auto">
        <thead>
        <tr>
            <!--            <ThSort {handler} orderBy="term">学期</ThSort>-->
            <!--            <ThSort {handler} orderBy="college">学院</ThSort>-->
            <!--            <ThSort {handler} orderBy="major">专业</ThSort>-->
            <ThSort {handler}>名次</ThSort>
            <ThSort {handler} orderBy="class">班级</ThSort>
            <ThSort {handler} orderBy="sno">学号</ThSort>
            <ThSort {handler} orderBy="name">姓名</ThSort>
            <ThSort {handler} orderBy="gpa">学分绩</ThSort>
        </tr>
        <tr>
            <!--            <ThFilter {handler} filterBy="term"></ThFilter>-->
            <!--            <ThFilter {handler} filterBy="college"></ThFilter>-->
            <!--            <ThFilter {handler} filterBy="major"></ThFilter>-->
            <ThFilter {handler} filterBy="ord"></ThFilter>
            <ThFilter {handler} filterBy="class"></ThFilter>
            <ThFilter {handler} filterBy="sno"></ThFilter>
            <ThFilter {handler} filterBy="name"></ThFilter>
            <ThFilter {handler} filterBy="gpa"></ThFilter>
        </tr>
        </thead>
        <tbody>
        {#each $rows as row}
            <tr>
                <!--                <td>{row.term}</td>-->
                <!--                <td>{row.college}</td>-->
                <!--                <td>{row.major}</td>-->
                <td>{row.ord}</td>
                <td>{row["class"]}</td>
                <td>{row.sno}</td>
                <td>{row.name}</td>
                <td>{row.gpa}</td>
            </tr>
        {/each}
        </tbody>
    </table>
    <!-- Footer -->
    <footer class="flex justify-between">
        <RowCount {handler}/>
        <Pagination {handler}/>
    </footer>
</div>