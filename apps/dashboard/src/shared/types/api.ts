/**
 * The reference response envelope every `apps/backend` endpoint returns
 * (`apps/backend/src/shared/core/response.core.ts`).
 */
export interface ApiResponse<T> {
  status: "success" | "fail" | "warning" | "error";
  success: boolean;
  message: string;
  data: T;
  statusCode?: number;
}

/** A paginated list response — the reference `ApiFeatures` query's counterpart. */
export interface Paginated<T> {
  status: "success" | "fail" | "warning" | "error";
  success: boolean;
  message: string;
  data: T[];
  page?: number;
  limit?: number;
  total?: number;
  totalPages?: number;
}

/** The reference `ApiFeatures` list query (page/limit/sort/search/searchBy + filters). */
export interface ApiFeaturesQuery {
  page?: number;
  limit?: number;
  sort?: string;
  fields?: string;
  search?: string;
  searchBy?: string;
  [filterKey: string]: string | number | undefined;
}
