<script setup lang="ts">
/**
 * `/dev/diagnostics` (18.B5) — dev-only inspector over the 5 log channels: Errors (grouped by
 * fingerprint), Performance (p50/p95 + budget breaches), Debug (namespace toggles + live tail),
 * Audit (business trail), Accounting (stub — Phase F). Reachable from the DevMenu section of
 * `NavUser.vue` and the command palette ("التشخيص"), same dev-only gate as `/dev/ui`.
 */
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/modules/core/components/shadcn/tabs';
import DevErrorsTab from '../components/dev/DevErrorsTab.vue';
import DevPerfTab from '../components/dev/DevPerfTab.vue';
import DevDebugTab from '../components/dev/DevDebugTab.vue';
import DevAuditTab from '../components/dev/DevAuditTab.vue';
import DevAccountingTab from '../components/dev/DevAccountingTab.vue';
</script>

<template>
  <div>
    <PageHeader title="التشخيص" subtitle="أخطاء، أداء، تتبع، تدقيق ومحاسبة — لأغراض التطوير فقط" />

    <Tabs default-value="errors">
      <TabsList>
        <TabsTrigger value="errors">الأخطاء</TabsTrigger>
        <TabsTrigger value="perf">الأداء</TabsTrigger>
        <TabsTrigger value="debug">التتبع</TabsTrigger>
        <TabsTrigger value="audit">التدقيق</TabsTrigger>
        <TabsTrigger value="accounting">المحاسبة</TabsTrigger>
      </TabsList>
      <TabsContent value="errors"><DevErrorsTab /></TabsContent>
      <TabsContent value="perf"><DevPerfTab /></TabsContent>
      <TabsContent value="debug"><DevDebugTab /></TabsContent>
      <TabsContent value="audit"><DevAuditTab /></TabsContent>
      <TabsContent value="accounting"><DevAccountingTab /></TabsContent>
    </Tabs>
  </div>
</template>
