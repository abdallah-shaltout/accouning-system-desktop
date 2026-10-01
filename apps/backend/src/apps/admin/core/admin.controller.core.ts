import { BaseController } from "@@shared/core/controller.core";
import type { BaseService } from "@@shared/core/service.core";
import type { Request, Response, NextFunction } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { adminActivityService } from "@@adminActivity/adminActivity.service";

/**
 * Admin CRUD base: every write is also recorded in admin_activity (docs/06-security.md).
 * `targetType` defaults to the module name; override per domain if it should differ.
 */
export class AdminBaseController<T extends BaseService<any> = BaseService<any>> extends BaseController<T> {
    targetType: string;

    constructor(service: T, moduleName = "السجل", targetType = moduleName) {
        super(service, moduleName);
        this.targetType = targetType;
    }

    createDocument = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const created = await this.Service.createDocument(req.body);
        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "create",
            targetType: this.targetType,
            targetId: (created as any).id,
            after: created,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });
        ApiResponse.success({ res, statusCode: 201, message: `تم إنشاء ${this.ModuleName} بنجاح`, data: created });
    }, "AdminBaseController.createDocument");

    updateDocument = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const before = await this.Service.readDocumentById(req.params.id);
        const updated = await this.Service.updateDocument({ id: req.params.id, data: req.body });
        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "update",
            targetType: this.targetType,
            targetId: req.params.id,
            before,
            after: updated,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });
        ApiResponse.success({ res, message: `تم تحديث ${this.ModuleName} بنجاح`, data: updated });
    }, "AdminBaseController.updateDocument");

    deleteDocument = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const before = await this.Service.readDocumentById(req.params.id);
        await this.Service.deleteDocument({ id: req.params.id });
        await adminActivityService.log({
            adminId: req.admin!.id,
            action: "delete",
            targetType: this.targetType,
            targetId: req.params.id,
            before,
            ip: req.ip,
            userAgent: req.headers["user-agent"],
        });
        ApiResponse.success({ res, message: `تم حذف ${this.ModuleName} بنجاح` });
    }, "AdminBaseController.deleteDocument");
}

export default AdminBaseController;
