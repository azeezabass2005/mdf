<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { getDocumentContent } from '$lib/db';
  import { blockSearchText, blockSearchKind, type SemanticData } from '$lib/types';
  import { fade, slide } from 'svelte/transition';

  let docId = $page.params.id;
  let document = $state<{ data: SemanticData; rawText?: string } | null>(null);
  let isLoading = $state(true);

  let showTools = $state(false);

  const fonts = [
    { id: 'inter', name: 'Inter', family: "'Inter', sans-serif" },
    { id: 'lora', name: 'Lora', family: "'Lora', serif" },
    { id: 'playfair', name: 'Playfair', family: "'Playfair Display', serif" },
    { id: 'jetbrains', name: 'Mono', family: "'JetBrains Mono', monospace" }
  ];
  let activeFont = $state(fonts[0]);

  const themes = [
    { id: 'dark', name: 'Dark', bg: 'bg-[#050505]', text: 'text-[#f4f4f5]', selection: 'selection:bg-white selection:text-black', isDark: true },
    { id: 'light', name: 'Light', bg: 'bg-[#fafafa]', text: 'text-[#0a0a0a]', selection: 'selection:bg-black selection:text-white', isDark: false },
    { id: 'nord', name: 'Nord', bg: 'bg-[#2e3440]', text: 'text-[#d8dee9]', selection: 'selection:bg-[#d8dee9] selection:text-[#2e3440]', isDark: true },
    { id: 'sepia', name: 'Sepia', bg: 'bg-[#f4ecd8]', text: 'text-[#433422]', selection: 'selection:bg-[#433422] selection:text-[#f4ecd8]', isDark: false }
  ];
  let activeTheme = $state(themes[0]);
  let isDarkMode = $derived(activeTheme.isDark);

  let isSearchOpen = $state(false);
  let searchQuery = $state('');
  let searchInput: HTMLInputElement | undefined = $state();

  let searchResults = $derived((() => {
    const doc = document;
    if (!searchQuery.trim() || !doc?.data?.pdf_semantic_data) return [];
    const lowerQuery = searchQuery.toLowerCase();
    const results: { id: string; text: string; page: number; kind: string }[] = [];
    doc.data.pdf_semantic_data.forEach((page, pIdx) => {
      page.forEach((block, bIdx) => {
        const text = blockSearchText(block);
        if (text && text.toLowerCase().includes(lowerQuery)) {
          results.push({
            id: `block-${pIdx}-${bIdx}`,
            text,
            page: pIdx + 1,
            kind: blockSearchKind(block),
          });
        }
      });
    });
    return results;
  })());

  onMount(async () => {
    if (!docId) return;
    document = await getDocumentContent(docId as string);
    isLoading = false;
  });

</script>

<svelte:head>
  <title>MDF Reader</title>
</svelte:head>

<div class="min-h-screen transition-colors duration-1000 {activeTheme.bg} {activeTheme.text} {activeTheme.selection}" style="font-family: {activeFont.family};">
  <div class="fixed top-0 left-0 w-full flex items-center justify-between px-6 py-6 z-[60] transition-opacity">
    <a href="/library" class="text-xs tracking-[0.3em] font-bold uppercase transition-opacity {isDarkMode ? 'text-white/50 hover:text-white' : 'text-black/50 hover:text-black'}">
      Library
    </a>
    <button onclick={() => { isSearchOpen = !isSearchOpen; if (isSearchOpen) setTimeout(() => searchInput?.focus(), 100); else searchQuery = ''; }} class="text-[10px] tracking-[0.3em] font-bold uppercase transition-colors {isDarkMode ? 'text-white/50 hover:text-white bg-white/5' : 'text-black/50 hover:text-black bg-black/5'} cursor-pointer px-4 py-2 rounded-full border {isDarkMode ? 'border-white/10 hover:border-white/30' : 'border-black/10 hover:border-black/30'}">
      {isSearchOpen ? 'Close Search' : 'Search Document'}
    </button>
  </div>

  {#if isSearchOpen}
     <div transition:slide={{duration:400, axis:'y'}} class="fixed top-0 left-0 w-full z-50 pt-28 pb-8 px-6 md:px-12 backdrop-blur-3xl {isDarkMode ? 'bg-[#050505]/95 border-b border-white/10' : 'bg-[#fafafa]/95 border-b border-black/10'} shadow-2xl flex flex-col items-center">
        <div class="w-full max-w-2xl relative">
           <input bind:this={searchInput} bind:value={searchQuery} type="text" placeholder="Search the document..." class="w-full text-2xl md:text-5xl font-medium tracking-tight bg-transparent border-none outline-none {isDarkMode ? 'text-white placeholder-white/20' : 'text-black placeholder-black/20'}" />
           <div class="w-full h-px {isDarkMode ? 'bg-white/20' : 'bg-black/20'} mt-6"></div>
        </div>
        {#if searchQuery.trim()}
           <div class="w-full max-w-2xl mt-8 max-h-[50vh] overflow-y-auto flex flex-col gap-2 scroll-smooth" style="scrollbar-width: none;">
              {#if searchResults.length === 0}
                 <p class="text-xs tracking-widest uppercase font-bold opacity-30 text-center py-12">No results found</p>
              {:else}
                 <div class="flex justify-between items-center mb-6">
                    <span class="text-[10px] tracking-[0.2em] font-bold uppercase {isDarkMode ? 'text-white/50' : 'text-black/50'}">{searchResults.length} Matches Found</span>
                 </div>
                 {#each searchResults.slice(0, 50) as result}
                    <button 
                      onclick={() => { 
                         const el = window.document.getElementById(result.id);
                         if (el) { 
                             el.scrollIntoView({ behavior: 'smooth', block: 'center' }); 
                             el.classList.add(isDarkMode ? 'bg-white/10' : 'bg-black/10');
                             setTimeout(() => el.classList.remove(isDarkMode ? 'bg-white/10' : 'bg-black/10'), 2000);
                         }
                         isSearchOpen = false;
                      }}
                      class="w-full text-left p-5 rounded-2xl {isDarkMode ? 'hover:bg-white/5 border-white/5' : 'hover:bg-black/5 border-black/5'} border transition-colors cursor-pointer flex flex-col gap-3 group/search"
                    >
                       <span class="text-[10px] uppercase tracking-[0.2em] font-bold {isDarkMode ? 'text-emerald-400 opacity-80 group-hover/search:opacity-100' : 'text-blue-600 opacity-80 group-hover/search:opacity-100'} transition-opacity">Page {result.page} <span class="{isDarkMode ? 'text-white/30' : 'text-black/30'} ml-3 hidden sm:inline-block">/ {result.kind}</span></span>
                       <p class="text-sm md:text-lg leading-[1.8] font-light opacity-90 truncate w-full" style="font-family: {activeFont.family};">{result.text}</p>
                    </button>
                 {/each}
              {/if}
           </div>
        {/if}
     </div>
  {/if}

  {#if isLoading}
     <div class="flex min-h-screen items-center justify-center">
        <span class="text-xs uppercase tracking-[0.3em] font-bold opacity-30 animate-pulse">Initializing Layout...</span>
     </div>
  {:else if !document}
     <div class="flex min-h-screen items-center justify-center">
        <div class="flex flex-col items-center gap-6">
           <span class="text-xs uppercase tracking-[0.2em] font-bold opacity-30">Document not found</span>
           <a href="/library" class="text-xs uppercase tracking-widest font-bold border-b {isDarkMode ? 'border-white/20 hover:border-white' : 'border-black/20 hover:border-black'} pb-1 transition-colors">Return to Library</a>
        </div>
     </div>
  {:else}
     <div class="max-w-3xl mx-auto px-6 pt-32 md:pt-48 pb-64 min-h-screen flex flex-col justify-center transition-all duration-1000" in:fade={{duration: 600}}>
        {#if document.data && document.data.pdf_semantic_data}
           {#each document.data.pdf_semantic_data as page, pIdx}
              <div class="mb-8">
                 {#each page as currBlock, bIdx}
                    {#if 'Text' in currBlock}
                       {@const textBlock = currBlock.Text}
                       {#if textBlock.kind === 'Title'}
                          <h1 id="block-{pIdx}-{bIdx}" class="text-4xl md:text-6xl font-medium tracking-tight mb-8 text-center text-balance scroll-mt-32 transition-colors duration-1000 p-2 rounded-xl">{textBlock.text}</h1>
                       {:else if textBlock.kind === 'Subtitle'}
                          <h2 id="block-{pIdx}-{bIdx}" class="text-xl md:text-2xl font-light opacity-80 mb-16 text-center text-balance scroll-mt-32 transition-colors duration-1000 p-2 rounded-xl">{textBlock.text}</h2>
                       {:else if textBlock.kind === 'Epigraph'}
                          <blockquote id="block-{pIdx}-{bIdx}" class="text-lg md:text-xl italic opacity-60 mb-12 text-center max-w-xl mx-auto text-balance scroll-mt-32 transition-colors duration-1000 p-2 rounded-xl">"{textBlock.text}"</blockquote>
                       {:else if textBlock.kind === 'Attribution'}
                          <p id="block-{pIdx}-{bIdx}" class="text-[10px] md:text-xs uppercase tracking-[0.3em] font-bold opacity-50 mb-16 text-center scroll-mt-32 transition-colors duration-1000 p-2 rounded-xl">{textBlock.text}</p>
                       {:else if textBlock.kind === 'Heading' || textBlock.kind === 'TableOfContentsHeading'}
                          <h3 id="block-{pIdx}-{bIdx}" class="text-2xl md:text-3xl font-medium tracking-tight mt-16 mb-6 scroll-mt-32 transition-colors duration-1000 p-2 rounded-xl">{textBlock.text}</h3>
                       {:else if textBlock.kind === 'ListItem'}
                          <div id="block-{pIdx}-{bIdx}" class="flex gap-6 mb-4 items-start ml-2 md:ml-4 scroll-mt-32 transition-colors duration-1000 p-2 rounded-xl">
                             <span class="opacity-30 flex-shrink-0 mt-1 text-xs px-2 border {isDarkMode ? 'border-white/20' : 'border-black/20'} rounded-full">/</span>
                             <p class="text-lg md:text-xl leading-[1.8] font-light opacity-90">{textBlock.text}</p>
                          </div>
                       {:else if textBlock.kind === 'SubListItem'}
                          <div id="block-{pIdx}-{bIdx}" class="flex gap-6 mb-3 items-start ml-12 md:ml-16 opacity-80 scroll-mt-32 transition-colors duration-1000 p-2 rounded-xl">
                             <span class="opacity-30 flex-shrink-0 mt-[10px] w-1 h-1 rounded-full {isDarkMode ? 'bg-white' : 'bg-black'}"></span>
                             <p class="text-[17px] md:text-lg leading-[1.8] font-light">{textBlock.text}</p>
                          </div>
                       {:else if textBlock.kind !== 'PageNumber'}
                          <p id="block-{pIdx}-{bIdx}" class="text-[19px] md:text-[21px] leading-[1.9] font-light mb-8 opacity-90 tracking-[-0.01em] text-pretty scroll-mt-32 transition-colors duration-1000 p-2 rounded-xl">
                             {#if textBlock.is_bold && textBlock.is_italic}
                                <strong class="italic font-bold">{textBlock.text}</strong>
                             {:else if textBlock.is_bold}
                                <strong class="font-bold font-sans tracking-tight">{textBlock.text}</strong>
                             {:else if textBlock.is_italic}
                                <em class="italic opacity-80">{textBlock.text}</em>
                             {:else}
                                {textBlock.text}
                             {/if}
                          </p>
                       {/if}
                    {:else}
                       <!-- TODO: backend does not emit Table blocks yet (see ContentBlock::Table in server/src/pdf_inference/reconstruct.rs). Scaffold renders the shape once it does. -->
                       {@const tableBlock = currBlock.Table}
                       <div id="block-{pIdx}-{bIdx}" class="mb-12 overflow-x-auto scroll-mt-32 transition-colors duration-1000 p-2 rounded-xl">
                          <table class="w-full border-collapse text-sm md:text-base font-light">
                             <tbody>
                                {#each tableBlock.rows as row}
                                   <tr class="border-b {isDarkMode ? 'border-white/10' : 'border-black/10'}">
                                      {#each row.cells as cell}
                                         <td
                                            colspan={cell.col_span > 1 ? cell.col_span : undefined}
                                            rowspan={cell.row_span > 1 ? cell.row_span : undefined}
                                            class="p-3 align-top leading-[1.6]"
                                         >
                                            {#if cell.is_bold && cell.is_italic}
                                               <strong class="italic font-bold">{cell.text}</strong>
                                            {:else if cell.is_bold}
                                               <strong class="font-bold">{cell.text}</strong>
                                            {:else if cell.is_italic}
                                               <em class="italic opacity-80">{cell.text}</em>
                                            {:else}
                                               {cell.text}
                                            {/if}
                                         </td>
                                      {/each}
                                   </tr>
                                {/each}
                             </tbody>
                          </table>
                       </div>
                    {/if}
                 {/each}
                 
                 <!-- Deterministic page boundary derived from data structure instead of semantic inference -->
                 {#if pIdx < document.data.pdf_semantic_data.length - 1}
                     <div class="w-full h-px {isDarkMode ? 'bg-white/10' : 'bg-black/10'} mt-24 mb-24 relative flex justify-center items-center transition-colors duration-1000">
                        <span class="absolute {activeTheme.bg} {isDarkMode ? 'text-white/30' : 'text-black/30'} px-6 text-[10px] font-sans tracking-[0.3em] font-bold uppercase transition-colors duration-1000">
                           Page {pIdx + 1}
                        </span>
                     </div>
                 {:else}
                     <div class="w-full mt-32 mb-12 flex justify-center items-center">
                        <span class="{isDarkMode ? 'text-white/20' : 'text-black/20'} text-[10px] font-sans tracking-[0.4em] font-bold uppercase">
                           End of Document
                        </span>
                     </div>
                 {/if}
              </div>
           {/each}
        {/if}
     </div>
  {/if}

  <div class="fixed bottom-6 right-6 md:bottom-8 md:right-8 z-50 flex flex-col items-end gap-6 font-sans">
    {#if showTools}
       <div transition:slide={{duration: 400, axis: 'y'}} class="w-[340px] max-w-[calc(100vw-3rem)] flex flex-col gap-2 p-2 rounded-[2rem] {isDarkMode ? 'bg-white/5 backdrop-blur-2xl border border-white/10 shadow-[0_0_40px_rgba(255,255,255,0.05)]' : 'bg-black/5 backdrop-blur-2xl border border-black/10 shadow-[0_0_40px_rgba(0,0,0,0.05)]'} shadow-2xl origin-bottom">
          <div class="flex items-center justify-between px-5 py-3 gap-12">
             <span class="text-[10px] tracking-[0.2em] uppercase font-bold {isDarkMode ? 'text-white/40' : 'text-black/40'}">Mode</span>
             <div class="flex gap-3">
                {#each themes as theme}
                  <button 
                    aria-label={theme.name} 
                    title={theme.name}
                    onclick={() => activeTheme = theme} 
                    class="w-6 h-6 rounded-full {theme.bg} border {isDarkMode ? 'border-white/20' : 'border-black/20'} {activeTheme.id === theme.id ? (isDarkMode ? '!border-white/80 scale-125' : '!border-black/80 scale-125') : 'opacity-60 hover:scale-110'} cursor-pointer transition-all shadow-inner"
                  ></button>
                {/each}
             </div>
          </div>
          <div class="flex flex-col px-5 py-4 gap-4 border-t {isDarkMode ? 'border-white/10' : 'border-black/10'}">
             <span class="text-[10px] tracking-[0.2em] uppercase font-bold {isDarkMode ? 'text-white/40' : 'text-black/40'}">Typography</span>
             <div class="flex gap-2 text-[13px] w-full overflow-x-auto pb-1" style="scrollbar-width: none;">
                {#each fonts as font}
                   <button 
                     onclick={() => activeFont = font} 
                     class="px-4 py-2 rounded-full flex items-center justify-center whitespace-nowrap bg-white/5 border {isDarkMode ? 'border-white/10' : 'border-black/5'} {activeFont.id === font.id ? (isDarkMode ? '!bg-white !text-black shadow-[0_0_10px_rgba(255,255,255,0.2)]' : '!bg-black !text-white shadow-[0_0_10px_rgba(0,0,0,0.2)]') : (isDarkMode ? 'text-white/50 hover:text-white' : 'text-black/50 hover:text-black')} transition-all cursor-pointer"
                     style="font-family: {font.family};"
                   >
                     {font.name}
                   </button>
                {/each}
             </div>
          </div>
       </div>
    {/if}
    <button aria-label="Toggle reader tools" onclick={() => showTools = !showTools} class="w-16 h-16 rounded-full flex items-center justify-center transition-transform hover:scale-105 shadow-2xl cursor-pointer {isDarkMode ? 'bg-white text-black' : 'bg-black text-white'} border border-white/5">
       <svg class="w-5 h-5 transition-transform duration-500 {showTools ? 'rotate-90' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <line x1="21" y1="4" x2="14" y2="4"></line>
          <line x1="10" y1="4" x2="3" y2="4"></line>
          <line x1="21" y1="12" x2="12" y2="12"></line>
          <line x1="8" y1="12" x2="3" y2="12"></line>
          <line x1="21" y1="20" x2="16" y2="20"></line>
          <line x1="12" y1="20" x2="3" y2="20"></line>
          <line x1="14" y1="1" x2="14" y2="7"></line>
          <line x1="8" y1="9" x2="8" y2="15"></line>
          <line x1="16" y1="17" x2="16" y2="23"></line>
       </svg>
    </button>
  </div>
</div>
