<script>
    import Sidebar from "./Sidebar.svelte";
    import "../app.css";
</script>

<div class="grid h-screen">
    <!--    <Header></Header>-->

    <!-- Grid Columns -->
    <div class="grid grid-cols-6">
        <!-- Sidebar -->
        <aside class="sticky top-0 col-span-1 h-screen">
            <Sidebar></Sidebar>
        </aside>
        <!-- Main Content -->
        <main class="col-span-5 p-4  mx-auto box-border">
            <slot></slot>
        </main>
    </div>

</div>


<style>
</style>
