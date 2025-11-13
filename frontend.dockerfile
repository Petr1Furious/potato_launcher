FROM node:20-alpine AS deps
WORKDIR /app

COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

FROM deps AS build
ENV NODE_ENV=production
COPY frontend ./
RUN npm run build

FROM nginx:1.27-alpine AS runtime

COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
