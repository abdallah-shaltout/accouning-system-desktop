<script setup lang="ts">
export interface LegalSection { title: string; body: string }
const props = defineProps<{ title: string; intro: string; updated: string; updatedLabel: string; sections: LegalSection[] }>()

useSeoMeta({ title: props.title, description: props.intro })
</script>

<template>
  <main id="main">
    <LPagePanel>
      <LContainer>
        <div class="mx-auto max-w-prose">
          <LBreadcrumb :items="[{ label: 'الرئيسية', to: '/' }, { label: title }]" />
          <h1 class="mt-10 text-h2 text-ink">{{ title }}</h1>
          <p class="mt-4 text-base text-ink-mute">آخر تحديث: <time :datetime="updated">{{ updatedLabel }}</time></p>
          <p class="mt-8 text-lg leading-loose text-ink-soft">{{ intro }}</p>
          <section v-for="s in sections" :key="s.title" class="mt-12">
            <h2 class="text-2xl text-ink">{{ s.title }}</h2>
            <p class="mt-4 text-lg leading-loose text-ink-soft">{{ s.body }}</p>
          </section>
        </div>
      </LContainer>
    </LPagePanel>
  </main>
</template>
