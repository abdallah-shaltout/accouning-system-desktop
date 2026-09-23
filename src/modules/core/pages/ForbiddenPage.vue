<script setup lang="ts">
import { useRoute } from 'vue-router';
import { House, Lock } from '@lucide/vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import AppButton from '../components/ui/AppButton.vue';
import EmptyState from '../components/ui/EmptyState.vue';
import { ROLE_LABEL } from '../helpers/labels';

const route = useRoute();
const auth = useAuthStore();
</script>

<template>
  <EmptyState
    :icon="Lock"
    title="ليست لديك صلاحية لهذه الصفحة"
    :description="`صلاحية ${auth.role ? ROLE_LABEL[auth.role] : ''} لا تسمح بالوصول إلى ${route.query.from ?? 'هذه الصفحة'}. تواصل مع مدير النظام إذا كنت تحتاجها.`"
  >
    <AppButton :icon="House" to="/">العودة للرئيسية</AppButton>
  </EmptyState>
</template>
