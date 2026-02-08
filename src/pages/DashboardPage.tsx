import { useMemo } from "react";
import type { ProjectListItem } from "../types/project";
import { Button } from "../components/ui/Button";
import { Icons } from "../components/ui/Icons";
import { ProjectCard } from "../components/ProjectCard";
import { formatDateTime } from "../utils/format";

type DashboardPageProps = {
  projects: ProjectListItem[];
  loading: boolean;
  error: string | null;
  onRefresh: () => Promise<void>;
  onNewRecording: () => void;
  onOpenProject: (projectId: string) => void;
  onDeleteProject: (projectId: string) => Promise<void>;
  onRenameProject: (projectId: string, title: string) => Promise<void>;
};

export function DashboardPage({
  projects,
  loading,
  error,
  onRefresh,
  onNewRecording,
  onOpenProject,
  onDeleteProject,
  onRenameProject
}: DashboardPageProps) {
  const latestProjectTime = useMemo(() => {
    if (!projects.length) {
      return "暂无项目";
    }
    return formatDateTime(projects[0].updatedAt);
  }, [projects]);

  return (
    <section className="dashboard-shell">
      <header className="dashboard-hero">
        <div>
          <p className="eyebrow">FocusLens Dashboard</p>
          <h1>项目概览</h1>
          <p className="muted">最近活跃：{latestProjectTime}</p>
        </div>
        <div className="dashboard-actions">
          <Button variant="outline" icon={<Icons.Refresh />} onClick={() => void onRefresh()} loading={loading}>
            刷新列表
          </Button>
          <Button variant="primary" icon={<Icons.Record />} onClick={onNewRecording}>
            开始新录制
          </Button>
        </div>
      </header>

      {error && <p className="error">{error}</p>}

      <div className="project-grid">
        {projects.map((project) => (
          <ProjectCard
            key={project.projectId}
            project={project}
            onOpen={onOpenProject}
            onDelete={onDeleteProject}
            onRename={onRenameProject}
          />
        ))}
      </div>

      {!loading && projects.length === 0 && (
        <div className="empty-state dashboard-empty">
          <div className="empty-state-icon">
            <Icons.Record size={80} />
          </div>
          <div className="empty-state-copy">
            <h3 className="empty-state-title">还没有任何录制</h3>
            <p className="muted empty-state-text">开启你的第一个 FocusLens 创作之旅吧</p>
          </div>
          <Button variant="primary" icon={<Icons.Record />} onClick={onNewRecording} className="empty-state-cta">
            立即开始录制
          </Button>
        </div>
      )}
    </section>
  );
}
