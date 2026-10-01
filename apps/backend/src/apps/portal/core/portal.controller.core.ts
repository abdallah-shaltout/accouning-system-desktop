import { BaseController } from "@@shared/core/controller.core";
import type { BaseService } from "@@shared/core/service.core";
import type { Request, Response, NextFunction } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";

/**
 * Portal CRUD base: every operation is scoped to req.orgId (multi-tenant isolation), the reference's
 * `StoreBaseController` with `storeId` renamed to `orgId`.
 */
export class PortalBaseController<T extends BaseService<any> = BaseService<any>> extends BaseController<T> {
    createDocument = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const created = await this.Service.createDocument({ ...req.body, orgId: req.orgId });
        ApiResponse.success({ res, statusCode: 201, message: `تم إنشاء ${this.ModuleName} بنجاح`, data: created });
    }, "PortalBaseController.createDocument");

    readDocument = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const result = await this.Service.readDocument({ query: req.query, reqFilter: { orgId: req.orgId } });
        ApiResponse.success({ res, data: result.data, page: result.page, limit: result.limit, total: result.total, totalPages: result.totalPages });
    }, "PortalBaseController.readDocument");
}

export default PortalBaseController;
