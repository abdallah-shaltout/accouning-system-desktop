import { Router } from "express";
import { PortalRequiredAuth, PortalAllowTo } from "@@shared/middleware/auth/portal.protacted";
import { portalUserController } from "./portal.user.controller";
import { portalUserValidation } from "./portal.user.validation";

const router = Router();

router.use(PortalRequiredAuth, PortalAllowTo("owner"));

router.get("/", portalUserValidation.list, portalUserController.list);
router.post("/", portalUserValidation.create, portalUserController.create);
router.delete("/:id", portalUserValidation.remove, portalUserController.remove);

export default router;
