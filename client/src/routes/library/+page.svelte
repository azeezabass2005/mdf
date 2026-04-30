<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { getAllDocuments, deleteDocument, saveDocument, type DocumentMeta } from '$lib/db';
  import { fade } from 'svelte/transition';

  let docs = $state<DocumentMeta[]>([]);
  let isLoading = $state(true);
  
  let isUploading = $state(false);
  let fileInput: HTMLInputElement | undefined = $state();

  function triggerFileInput() {
    fileInput?.click();
  }

  async function handleFileInput(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files && input.files.length > 0) {
      const file = input.files[0];
      if (file.type !== "application/pdf") {
        alert("Please upload a valid PDF file.");
        return;
      }
      
      isUploading = true;
      try {
        const formData = new FormData();
        formData.append('file', file);
        const response = await fetch('http://localhost:4000/infer_semantics', {
          method: 'POST',
          body: formData
        });
        if (!response.ok) throw new Error("Upload failed");
        const semanticData = await response.json();
        const docId = await saveDocument(file.name, file.size, semanticData);
        await goto(`/reader/${docId}`);
      } catch (err) {
        console.error(err);
        alert("Upload failed. Try again.");
        isUploading = false;
      }
      input.value = "";
    }
  }

  onMount(async () => {
    docs = await getAllDocuments();
    isLoading = false;
  });

  async function handleDelete(id: string) {
    if(!confirm("Are you sure you want to delete this document?")) return;
    await deleteDocument(id);
    docs = docs.filter(d => d.id !== id);
  }

  function formatDate(ts: number) {
     return new Date(ts).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  }
</script>

<div class="min-h-screen bg-[#050505] text-white flex flex-col pt-32 pb-24 px-6 md:px-12 selection:bg-white selection:text-black">
  <div class="fixed top-0 left-0 w-full flex items-center justify-between px-6 py-6 z-50 mix-blend-difference border-b border-white/10 backdrop-blur-md">
    <a href="/" class="text-xl font-bold tracking-[0.2em] uppercase leading-none hover:opacity-50 transition-opacity">mdf</a>
    <span class="hidden sm:block text-[10px] uppercase tracking-[0.3em] font-bold text-white/50">/ Library</span>
    <button onclick={triggerFileInput} class="text-[10px] uppercase tracking-[0.3em] font-bold {isUploading ? 'text-emerald-400 animate-pulse' : 'text-white/50 hover:text-white'} transition-colors cursor-pointer disabled:opacity-50" disabled={isUploading}>
       {isUploading ? 'Reconstructing...' : 'Upload New'}
    </button>
    <input bind:this={fileInput} type="file" accept="application/pdf" class="hidden" onchange={handleFileInput} />
  </div>

  <div class="max-w-5xl w-full mx-auto flex-1 flex flex-col">
    <div class="mb-16 md:mb-24">
      <h1 class="text-5xl md:text-8xl tracking-tighter uppercase font-medium mb-4">Archive</h1>
      <p class="text-white/40 font-light tracking-wide text-lg">Your reconstructed semantics.</p>
    </div>

    {#if isLoading}
      <div class="flex-1 flex items-center justify-center border border-white/10 py-32">
         <span class="text-xs uppercase tracking-widest font-bold text-white/30 animate-pulse">Retrieving Indexes...</span>
      </div>
    {:else if docs.length === 0}
      <div class="flex-1 flex flex-col items-center justify-center border border-white/10 p-12 py-32 text-center group hover:bg-white/[0.01]">
         <span class="text-6xl md:text-8xl font-serif italic text-white/10 mb-8 transition-colors group-hover:text-white/20">0</span>
         <span class="text-sm uppercase tracking-[0.2em] font-bold text-white/30 mb-4">No documents found</span>
         <button onclick={triggerFileInput} class="text-white hover:text-white/50 transition-colors underline decoration-white/20 underline-offset-4 cursor-pointer disabled:opacity-50" disabled={isUploading}>
            {isUploading ? 'Reconstructing...' : 'Convert your first PDF'}
         </button>
      </div>
    {:else}
      <div class="border-t border-white/10">
        {#each docs as doc (doc.id)}
          <div transition:fade={{duration: 400}} class="group flex flex-col md:flex-row md:items-center justify-between p-6 md:p-8 border-b border-white/10 hover:bg-white/[0.02] transition-colors gap-6 md:gap-0">
            
            <div class="flex flex-col">
              <a href="/reader/{doc.id}" class="text-2xl md:text-3xl tracking-tight font-medium mb-2 group-hover:text-emerald-400 transition-colors w-full md:w-[400px] lg:w-[500px] truncate block" title={doc.fileName}>
                {doc.fileName}
              </a>
              <div class="flex items-center gap-4 text-xs font-bold tracking-widest text-white/30 uppercase">
                 <span>{(doc.size / 1024 / 1024).toFixed(2)} MB</span>
                 <span class="w-1 h-1 rounded-full bg-white/20"></span>
                 <span>{formatDate(doc.createdAt)}</span>
              </div>
            </div>

            <div class="flex items-center gap-4 text-xs font-bold tracking-widest uppercase">
               <a href="/reader/{doc.id}" class="px-6 py-3 border border-white/20 hover:bg-white hover:text-black transition-colors block text-center min-w-[100px]">Read</a>
               <button onclick={() => handleDelete(doc.id)} class="px-6 py-3 border border-red-500/20 text-red-500 hover:bg-red-500 hover:text-white transition-colors cursor-pointer min-w-[100px]">
                 Delete
               </button>
            </div>
            
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
