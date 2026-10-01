<script setup lang="ts">
import { ref } from "vue";
import { useForm } from "vee-validate";
import { toTypedSchema } from "@vee-validate/zod";
import { useRoute, useRouter } from "vue-router";
import { toast } from "vue-sonner";
import { AppButton, FormField } from "@/shared/components/app";
import { ApiError, errorMessageFor } from "@/shared/api/errors";
import { portalLoginSchema } from "../../schemas/portalAuth.schema";
import { authService } from "../../services/authService";

/** Portal login: phone + password (docs/04-screens.md `portal-login`). */
const router = useRouter();
const route = useRoute();
const submitting = ref(false);

const form = useForm({ validationSchema: toTypedSchema(portalLoginSchema) });

const submit = form.handleSubmit(async (values) => {
  submitting.value = true;
  try {
    await authService.portalLogin(values);
    const redirect = typeof route.query.redirect === "string" ? route.query.redirect : undefined;
    if (redirect) {
      await router.push(redirect); /* route-ok: login redirect query is a URL round-trip, not a hard-coded target */
    } else {
      await router.push({ name: "portal-home" });
    }
  } catch (err) {
    toast.error(errorMessageFor(err instanceof ApiError ? err : null));
  } finally {
    submitting.value = false;
  }
});
</script>

<template>
  <form class="flex flex-col gap-4" @submit="submit">
    <h2 class="text-lead font-semibold text-text-primary">تسجيل الدخول</h2>
    <FormField name="phone" label="رقم الهاتف" type="tel" autocomplete="tel" dir="ltr" />
    <FormField name="password" label="كلمة المرور" type="password" autocomplete="current-password" />
    <AppButton type="submit" class="w-full" :loading="submitting">دخول</AppButton>
  </form>
</template>
