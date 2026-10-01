import express from "express";
import appUse from "@@config/appUse";

const app = express();
appUse(app);

export default app;
