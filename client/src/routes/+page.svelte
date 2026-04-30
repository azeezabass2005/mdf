<script lang="ts">
  import { goto } from '$app/navigation';
  import { PUBLIC_API_BASE_URL } from '$env/static/public';
  import { saveDocument } from '$lib/db';

  let isDragging = $state(false);
  let selectedFile = $state<File | null>(null);
  let isUploading = $state(false);
  let uploadError = $state<string | null>(null);

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave() {
    isDragging = false;
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
    if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      if (file.type === "application/pdf") {
        selectedFile = file;
      } else {
        alert("Please upload a valid PDF file.");
      }
    }
  }

  let fileInput: HTMLInputElement | undefined = $state();

  function triggerFileInput() {
    fileInput?.click();
  }

  function handleFileInput(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files && input.files.length > 0) {
      const file = input.files[0];
      if (file.type === "application/pdf") {
        selectedFile = file;
      } else {
        alert("Please upload a valid PDF file.");
      }
    }
  }

  function scrollToSection(e: MouseEvent, id: string) {
    e.preventDefault();
    const element = document.getElementById(id);
    if (element) {
      element.scrollIntoView({ behavior: 'smooth' });
    }
  }

  async function handleUpload() {
    if (!selectedFile) return;
    
    isUploading = true;
    uploadError = null;

    try {
      const formData = new FormData();
      formData.append('file', selectedFile);

      const response = await fetch(`${PUBLIC_API_BASE_URL}/infer_semantics`, {
        method: 'POST',
        body: formData
      });

      if (!response.ok) {
        throw new Error(`Upload failed: ${response.statusText}`);
      }

      const semanticData = await response.json();
      
      const docId = await saveDocument(selectedFile.name, selectedFile.size, semanticData);
      
      await goto(`/reader/${docId}`);
      
    } catch (err: any) {
      console.error("Upload error:", err);
      uploadError = err.message || "Failed to process document";
      isUploading = false;
    }
  }
</script>

<div class="relative min-h-screen bg-[#050505] text-[#f4f4f5] selection:bg-[#f4f4f5] selection:text-[#050505] font-sans overflow-x-hidden antialiased">
  
  <header class="fixed top-0 w-full flex items-center justify-between px-6 py-6 z-50 mix-blend-difference border-b border-white/10 backdrop-blur-md">
    <div class="flex items-end gap-4">
      <div class="text-xl font-bold tracking-[0.2em] uppercase leading-none">mdf</div>
      <div class="hidden sm:block text-[10px] uppercase tracking-[0.3em] font-bold text-white/50 mb-[2px]">
        ( Maldives for PDFs )
      </div>
    </div>
    <div class="flex items-center gap-6 md:gap-8">
      <div class="flex gap-5 md:gap-6 text-[10px] md:text-xs uppercase tracking-[0.3em] font-bold text-white/50">
        <a href="/library" class="hover:text-white transition-colors">Library</a>
        <a href="#features" onclick={(e) => scrollToSection(e, 'features')} class="hover:text-white transition-colors hidden md:block">Features</a>
      </div>
      
      <a href="https://github.com/azeezabass2005/mdf" target="_blank" class="text-white/50 hover:text-white transition-colors flex items-center gap-2">
         <span class="hidden md:block text-xs uppercase tracking-[0.3em] font-bold">GitHub</span>
         <svg class="w-5 h-5 md:hidden" viewBox="0 0 24 24" fill="currentColor">
           <path d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z"/>
         </svg>
      </a>

      <button onclick={triggerFileInput} class="relative overflow-hidden group border border-white/20 rounded-full px-6 py-2 hidden sm:block cursor-pointer">
        <span class="relative z-10 text-xs tracking-widest uppercase font-bold group-hover:text-black transition-colors duration-500">Upload</span>
        <div class="absolute inset-0 bg-white translate-y-full group-hover:translate-y-0 transition-transform duration-500 ease-[cubic-bezier(0.19,1,0.22,1)]"></div>
      </button>
    </div>
  </header>

  <section class="relative min-h-screen flex flex-col justify-center pt-24 pb-8 px-6 md:px-12 z-10 border-b border-white/10">
    <div class="absolute top-1/2 -translate-y-1/2 left-0 w-[200vw] overflow-hidden pointer-events-none select-none opacity-[0.02] flex items-center">
      <h2 class="text-[30vw] leading-none whitespace-nowrap font-medium tracking-tighter uppercase marquee-text">
        MALDIVES FOR PDFS MALDIVES FOR PDFS MALDIVES FOR PDFS
      </h2>
    </div>

    <div class="relative z-10 grid grid-cols-1 lg:grid-cols-12 gap-12 w-full max-w-none items-center mt-12 md:mt-16">
      
      <div class="lg:col-span-7 flex flex-col justify-center">
        <h1 class="text-[clamp(4rem,10vw,12rem)] leading-[0.8] tracking-tighter uppercase font-medium">
          <span class="block overflow-hidden pb-2"><span class="block animate-slide-up">Liberate</span></span>
          <span class="block overflow-hidden pb-4"><span class="block animate-slide-up delay-100 font-serif italic text-white/40 tracking-tight">Your</span></span>
          <span class="block overflow-hidden pb-2"><span class="block animate-slide-up delay-200">Documents.</span></span>
        </h1>
        <div class="w-full h-px bg-white/20 mt-12 mb-8 origin-left animate-scale-x delay-500 relative">
          <div class="absolute right-0 top-1/2 -translate-y-1/2 w-2 h-2 rounded-full bg-white"></div>
        </div>
        <p class="max-w-xl text-lg font-light leading-relaxed text-white/50 tracking-wide animate-fade-in delay-700">
          Stop zooming and panning just to read a document. MDF convert your PDFs into clean, responsive web pages you'll actually enjoy reading.
        </p>
      </div>

      <div class="lg:col-span-5 aspect-square max-h-[500px] lg:max-h-none lg:aspect-auto lg:h-[400px] w-full mt-12 lg:mt-0 relative group perspective-1000">
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div 
          class="absolute inset-0 w-full h-full flex flex-col justify-between border border-white/10 bg-white/[0.01] backdrop-blur-sm p-8 transition-all duration-700 ease-[cubic-bezier(0.16,1,0.3,1)]
            {isDragging ? 'border-white/50 bg-white/5 scale-[0.98]' : 'hover:bg-white/[0.03] hover:border-white/20 hover:scale-[1.02]'}
            overflow-hidden group"
          ondragover={handleDragOver}
          ondragleave={handleDragLeave}
          ondrop={handleDrop}
        >
          <div class="absolute top-0 left-0 w-4 h-4 border-t border-l border-white/50"></div>
          <div class="absolute top-0 right-0 w-4 h-4 border-t border-r border-white/50"></div>
          <div class="absolute bottom-0 left-0 w-4 h-4 border-b border-l border-white/50"></div>
          <div class="absolute bottom-0 right-0 w-4 h-4 border-b border-r border-white/50"></div>

          {#if selectedFile}
            <button class="absolute top-6 right-6 z-20 text-[10px] cursor-pointer uppercase tracking-widest font-bold text-white/30 hover:text-white transition-colors" onclick={() => selectedFile = null}>
              Change File
            </button>
            <div class="h-full flex flex-col justify-end animate-fade-in relative z-10 w-full">
              <span class="text-xs uppercase tracking-[0.3em] font-bold text-emerald-400/80 mb-4 block">Document Ready</span>
              <span class="text-3xl md:text-5xl tracking-tight mb-2 truncate max-w-full block" title={selectedFile.name}>{selectedFile.name}</span>
              <span class="text-sm font-serif italic text-white/50 mb-12 block">
                [ {(selectedFile.size / 1024 / 1024).toFixed(3)} MB ]
              </span>
              {#if uploadError}
                 <p class="text-red-500 text-xs tracking-widest uppercase font-bold mb-4">{uploadError}</p>
              {/if}
              <button 
                class="relative cursor-pointer w-full overflow-hidden bg-white text-black py-5 px-6 group/btn {isUploading ? 'opacity-50 pointer-events-none' : ''}"
                onclick={handleUpload}
              >
                 <span class="relative z-10 uppercase tracking-[0.2em] text-xs font-bold transition-all group-hover/btn:text-white">
                    {isUploading ? 'Processing Document...' : 'Start Reading'}
                 </span>
                 <div class="absolute bottom-0 left-0 w-full h-0 bg-black transition-all duration-500 ease-[cubic-bezier(0.19,1,0.22,1)] group-hover/btn:h-full"></div>
              </button>
            </div>
          {:else}
            <div class="flex justify-between items-start">
               <span class="text-xs tracking-[0.2em] font-light uppercase text-white/30 {isDragging ? 'text-white' : ''} transition-colors">( Drop Zone )</span>
               <div class="w-2 h-2 rounded-full bg-white/20 {isDragging ? 'animate-ping !bg-white' : ''}"></div>
            </div>

            <div class="text-center md:text-left mt-auto">
               <span class="block text-2xl md:text-4xl tracking-tighter font-medium mb-6 transition-all {isDragging ? 'text-white translate-y-2' : 'text-white/60'}">
                 Awaiting your <br />
                 <span class="font-serif italic font-light {isDragging ? 'text-white' : 'text-white/30'}">document...</span>
               </span>
               <div class="relative inline-block mt-4 overflow-hidden group/btn">
                  <div class="absolute left-0 bottom-0 w-full h-px bg-white scale-x-0 group-hover/btn:scale-x-100 transition-transform origin-left duration-500"></div>
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <span class="relative uppercase tracking-widest text-xs font-bold pb-2 cursor-pointer transition-colors hover:text-white" onclick={triggerFileInput}>
                    Select via Dialog
                  </span>
                  <input 
                    bind:this={fileInput}
                    type="file" 
                    accept="application/pdf"
                    class="hidden"
                    onchange={handleFileInput}
                  />
               </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </section>

  <section id="features" class="relative z-10 w-full bg-[#050505] scroll-mt-28">
    <div class="max-w-[100vw] mx-auto hidden lg:block border-b border-white/10">
      <div class="grid grid-cols-12 auto-rows-[minmax(8rem,auto)]">
        
        <div class="col-span-8 border-b border-r border-white/10 p-16 flex flex-col justify-between hover:bg-white/[0.02] transition-colors duration-500 min-h-[500px]">
          <div class="flex justify-between items-start">
            <span class="text-8xl font-serif italic text-white/10 hover:text-white/20 transition-colors">/0 1</span>
            <span class="uppercase text-xs tracking-[0.3em] font-bold text-white/30">Architecture</span>
          </div>
          <div>
            <h3 class="text-4xl tracking-tighter uppercase font-medium mb-6">Beautiful Reading</h3>
            <p class="max-w-xl text-white/40 text-lg font-light leading-relaxed hover:text-white/60 transition-colors">
              Beautiful fonts, perfect spacing, and clean layouts. We redesign your documents specifically for comfortable reading on any screen.
            </p>
          </div>
        </div>
        
        <div class="col-span-4 border-b border-white/10 bg-white/[0.01] overflow-hidden flex items-center justify-center p-12">
            <div class="w-full max-w-[200px] flex flex-col gap-5 opacity-30 hover:opacity-60 transition-opacity duration-1000">
               <div class="w-full h-px bg-white"></div>
               <div class="w-[85%] h-px bg-white"></div>
               <div class="w-[90%] h-px bg-white"></div>
               <div class="w-[60%] h-px bg-white"></div>
               
               <div class="w-full h-8 mt-4 border border-white/40 flex items-center px-4">
                  <div class="w-1/3 h-px bg-white/80"></div>
               </div>
               
               <div class="w-full flex justify-between mt-6">
                  <div class="w-8 h-8 rounded-full border border-white/40 flex items-center justify-center">
                     <div class="w-2 h-2 rounded-full bg-white/80"></div>
                  </div>
                  <div class="flex flex-col justify-center gap-3 w-[60%]">
                     <div class="w-full h-px bg-white"></div>
                     <div class="w-[40%] h-px bg-white"></div>
                  </div>
               </div>
            </div>
        </div>

        <div class="col-span-4 border-r border-white/10 p-12 flex items-end">
           <p class="uppercase text-[10px] tracking-widest text-white/30 font-bold max-w-[200px] leading-relaxed">
             Built for phones, tablets & desktop.
           </p>
        </div>

        <div class="col-span-8 border-b border-white/10 p-16 flex flex-col justify-between hover:bg-white/[0.02] transition-colors duration-500 min-h-[500px]">
          <div class="flex justify-between items-start">
            <span class="text-8xl font-serif italic text-white/10 hover:text-white/20 transition-colors">/0 2</span>
            <span class="uppercase text-xs tracking-[0.3em] font-bold text-white/30">Focus</span>
          </div>
          <div>
            <h3 class="text-4xl tracking-tighter uppercase font-medium mb-6">Distraction Free</h3>
            <p class="max-w-xl text-white/40 text-lg font-light leading-relaxed hover:text-white/60 transition-colors">
              We strip away the clunky PDF interface and extract just your content. Pure text, images, and nothing else getting in your way.
            </p>
          </div>
        </div>

      </div>
    </div>

    <div class="block lg:hidden border-b border-white/10">
      <div class="flex flex-col">
        <div class="border-b border-white/10 p-6 py-12 hover:bg-white/[0.02]">
           <span class="text-5xl font-serif italic text-white/10 block mb-8">0 1</span>
           <h3 class="text-xl tracking-tighter uppercase font-medium mb-3">Beautiful Reading</h3>
           <p class="text-white/40 font-light leading-relaxed text-sm">Beautiful fonts, perfect spacing, and clean layouts. Designed for comfortable reading.</p>
        </div>
        <div class="border-b border-white/10 p-6 py-12 hover:bg-white/[0.02]">
           <span class="text-5xl font-serif italic text-white/10 block mb-8">0 2</span>
           <h3 class="text-xl tracking-tighter uppercase font-medium mb-3">Distraction Free</h3>
           <p class="text-white/40 font-light leading-relaxed text-sm">We strip away the clunky interface and extract just your content. Pure text and images.</p>
        </div>
      </div>
    </div>
  </section>

  <section id="process" class="py-24 md:py-48 px-4 md:px-6 overflow-hidden border-b border-white/10 bg-[#050505] flex flex-col justify-center items-center relative scroll-mt-20 group/section cursor-default">
      
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="relative z-10 text-center max-w-5xl mx-auto flex flex-col items-center group/text cursor-pointer" onclick={() => window.scrollTo({top: 0, behavior: 'smooth'})}>
         <span class="block text-white/30 italic font-serif text-3xl md:text-[4vw] mb-4 md:mb-6 font-light group-hover/text:text-white/60 transition-colors duration-700">Experience the</span>
         
         <h2 class="text-[15vw] md:text-[10vw] tracking-tighter uppercase font-bold text-center leading-none text-transparent transition-all duration-700 [-webkit-text-stroke:1px_rgba(255,255,255,0.2)] group-hover/text:[-webkit-text-stroke:0px_transparent] group-hover/text:text-white select-none">
            Difference
         </h2>
         <div class="w-0 h-px bg-white group-hover/text:w-full transition-all duration-700 ease-[cubic-bezier(0.16,1,0.3,1)] opacity-0 group-hover/text:opacity-100 mt-8"></div>
         
         <svg class="mt-8 md:mt-12 w-8 h-8 md:w-12 md:h-12 text-white/20 group-hover/text:text-white transform group-hover/text:-translate-y-4 transition-all duration-700" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M12 19V5M5 12l7-7 7 7" stroke-linecap="round" stroke-linejoin="round"/>
         </svg>
      </div>
  </section>

  <footer class="pt-12 md:pt-24 pb-8 px-6 lg:px-12 bg-[#050505] relative z-10 w-full">
    <div class="flex flex-col lg:flex-row justify-between lg:items-end gap-10 lg:gap-16 mb-12 lg:mb-24">
      <div class="max-w-sm">
         <h4 class="text-2xl md:text-xl tracking-tighter uppercase font-bold mb-4">Maldives for PDFs</h4>
         <p class="text-white/40 font-light leading-relaxed text-sm md:text-base">
           The future of digital document consumption. Stripping away the friction of rigid formats for a pure reading experience.
         </p>
      </div>
      
      <div class="grid grid-cols-2 mt-4 lg:mt-0 gap-8 md:gap-12 text-xs font-bold tracking-[0.2em] uppercase">
         <!-- svelte-ignore a11y_invalid_attribute -->
         <div class="flex flex-col gap-4">
            <span class="text-white/30 mb-1 md:mb-2">Connect</span>
            <a href="https://github.com/azeezabass2005/mdf" target="_blank" class="hover:text-white text-white/60 transition-colors">GitHub</a>
            <a href="#" class="hover:text-white text-white/60 transition-colors">Follow</a>
         </div>
         <!-- svelte-ignore a11y_invalid_attribute -->
         <div class="flex flex-col gap-4">
            <span class="text-white/30 mb-1 md:mb-2">Legal</span>
            <a href="#" class="hover:text-white text-white/60 transition-colors">Imprint</a>
            <a href="#" class="hover:text-white text-white/60 transition-colors">Privacy</a>
         </div>
      </div>
    </div>
    
    <div class="w-full text-center overflow-hidden flex items-center justify-center pointer-events-none select-none border-t border-white/10 pt-12 pb-8 md:pb-0">
       <span class="text-[25vw] leading-[0.7] tracking-tight font-medium text-white/[0.03]">
         MDF
       </span>
    </div>
    
    <div class="mt-8 md:mt-12 flex flex-col md:flex-row justify-center md:justify-between items-center gap-6 md:gap-0 text-[10px] tracking-[0.3em] uppercase font-bold text-white/30 text-center">
      <span>© 2026 // ALL RIGHTS RESERVED</span>
      <span>SYSTEM // ONLINE</span>
    </div>
  </footer>
</div>

<style>
  @keyframes marquee {
    0% { transform: translate3d(5%, 0, 0); }
    100% { transform: translate3d(-100%, 0, 0); }
  }
  .marquee-text {
    animation: marquee 60s linear infinite;
    will-change: transform;
  }
  
  @keyframes slide-up {
    0% { transform: translateY(120%); opacity: 0; }
    100% { transform: translateY(0); opacity: 1; }
  }
  .animate-slide-up {
    animation: slide-up 1s cubic-bezier(0.16, 1, 0.3, 1) forwards;
    transform: translateY(120%);
  }

  @keyframes scale-x {
    0% { transform: scaleX(0); }
    100% { transform: scaleX(1); }
  }
  .animate-scale-x {
    animation: scale-x 1.2s cubic-bezier(0.16, 1, 0.3, 1) forwards;
    transform: scaleX(0);
  }

  @keyframes fade-in {
    0% { opacity: 0; }
    100% { opacity: 1; }
  }
  .animate-fade-in {
    animation: fade-in 1s ease-out forwards;
    opacity: 0;
  }

  .delay-100 { animation-delay: 100ms; }
  .delay-200 { animation-delay: 200ms; }
  .delay-500 { animation-delay: 500ms; }
  .delay-700 { animation-delay: 700ms; }
</style>