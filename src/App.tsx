import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import { invoke, convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { appDataDir, join } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";

interface PhotoItem {
  path: string;
  file_name: string;
}

interface SpeciesNode {
  latin: string;
  chinese: string;
  count: number;
  photos: PhotoItem[];
}

interface GenusNode {
  name: string;
  count: number;
  species: SpeciesNode[];
}

interface FamilyNode {
  name: string;
  count: number;
  genera: GenusNode[];
}

interface OrderNode {
  name: string;
  count: number;
  families: FamilyNode[];
}

interface TaxonTree {
  orders: OrderNode[];
}

interface ScanStats {
  total_files: number;
  matched_files: number;
  unmatched_files: number;
}

interface ScanResponse {
  tree: TaxonTree;
  stats: ScanStats;
  total_species: number;
}

function toThumbnailSrc(path: string): string {
  if (!path) return "";
  try {
    return isTauri() ? convertFileSrc(path) : path;
  } catch {
    return path;
  }
}

export default function App() {
  const [iocPath, setIocPath] = useState("Multiling IOC 15.1_d.xlsx");
  const [cachePath, setCachePath] = useState("");
  const [roots, setRoots] = useState<string[]>([]);
  const [scanResult, setScanResult] = useState<ScanResponse | null>(null);
  const [treeQuery, setTreeQuery] = useState("");
  const [selectedSpecies, setSelectedSpecies] = useState<SpeciesNode | null>(null);
  const [selectedPhoto, setSelectedPhoto] = useState<PhotoItem | null>(null);
  const [thumbnailErrorMap, setThumbnailErrorMap] = useState<
    Record<string, boolean>
  >({});
  const [isScanning, setIsScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const rootPickerInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    appDataDir()
      .then((dir) => join(dir, "birdindex2", "cache.json"))
      .then((path) => setCachePath(path))
      .catch(() => setCachePath(""));
  }, []);

  useEffect(() => {
    const input = rootPickerInputRef.current;
    if (!input) return;
    input.setAttribute("webkitdirectory", "");
    input.setAttribute("directory", "");
  }, []);

  useEffect(() => {
    setThumbnailErrorMap({});
  }, [selectedSpecies]);

  const rootsLabel = useMemo(() => {
    if (roots.length === 0) return "0 个目录";
    return `${roots.length} 个目录`;
  }, [roots.length]);

  const handleBrowserRootInputChange = (event: ChangeEvent<HTMLInputElement>) => {
    const files = event.target.files;
    if (!files) return;
    if (files.length === 0) {
      event.target.value = "";
      return;
    }

    const parsedRoots = new Set<string>();
    for (const file of Array.from(files)) {
      const relativePath = file.webkitRelativePath;
      if (!relativePath) continue;
      const [topLevel] = relativePath.split("/");
      if (topLevel) parsedRoots.add(topLevel);
    }

    if (parsedRoots.size === 0) {
      setError("未能从浏览器选择结果中解析目录，请在 Tauri 桌面应用中运行。");
      event.target.value = "";
      return;
    }

    setError(null);
    setRoots((prev) => Array.from(new Set([...prev, ...Array.from(parsedRoots)])));
    event.target.value = "";
  };

  const handlePickRoots = async () => {
    setError(null);

    if (!isTauri()) {
      rootPickerInputRef.current?.click();
      return;
    }

    try {
      const selected = await open({
        directory: true,
        multiple: true,
        title: "选择根目录"
      });
      if (!selected) return;
      const next = Array.isArray(selected) ? selected : [selected];
      setRoots((prev) => Array.from(new Set([...prev, ...next])));
    } catch (err) {
      setError(`打开目录选择失败：${String(err)}`);
    }
  };

  const handleRemoveRoot = (path: string) => {
    setRoots((prev) => prev.filter((item) => item !== path));
  };

  const handleClearRoots = () => {
    setRoots([]);
  };

  const handleScan = async () => {
    if (!isTauri()) {
      setError("当前为浏览器模式，无法执行本地扫描，请在 Tauri 桌面应用中运行。");
      return;
    }

    setIsScanning(true);
    setError(null);
    setSelectedSpecies(null);
    setSelectedPhoto(null);
    try {
      const effectiveCachePath = cachePath || "cache.json";
      const response = await invoke<ScanResponse>("scan", {
        request: {
          roots,
          ioc_path: iocPath,
          cache_path: effectiveCachePath
        }
      });
      setScanResult(response);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsScanning(false);
    }
  };

  const handleReveal = async () => {
    if (!selectedPhoto) return;
    try {
      await invoke("reveal", { path: selectedPhoto.path });
    } catch (err) {
      setError(String(err));
    }
  };

  const handleOpen = async (path: string) => {
    try {
      await invoke("open_file", { path });
    } catch (err) {
      setError(String(err));
    }
  };

  const handleThumbnailError = (path: string) => {
    setThumbnailErrorMap((prev) => {
      if (prev[path]) return prev;
      return { ...prev, [path]: true };
    });
  };

  return (
    <div className="app-shell">
      <header className="toolbar">
        <div>
          <h1>BirdIndex2</h1>
          <p>Local-only IOC indexing for bird photos.</p>
        </div>
        <div className="toolbar-actions">
          <button
            className="primary"
            onClick={handleScan}
            disabled={isScanning || roots.length === 0 || !iocPath}
          >
            {isScanning ? "Scanning..." : "Scan"}
          </button>
        </div>
      </header>

      <section className="panel settings">
        <label>
          IOC 文件路径
          <input
            value={iocPath}
            onChange={(event) => setIocPath(event.target.value)}
            placeholder="Multiling IOC 15.1_d.xlsx"
          />
        </label>
        <div className="root-picker">
          <div className="root-header">
            <div>
              <div className="root-title">扫描根目录</div>
              <div className="root-subtitle">{rootsLabel}</div>
            </div>
            <div className="root-actions">
              <button className="ghost" onClick={handlePickRoots}>
                选择目录
              </button>
              <button className="ghost" onClick={handleClearRoots}>
                清空
              </button>
            </div>
            <input
              ref={rootPickerInputRef}
              type="file"
              multiple
              style={{ display: "none" }}
              onChange={handleBrowserRootInputChange}
            />
          </div>
          <div className="root-list">
            {roots.length === 0 ? (
              <div className="empty">尚未选择任何目录</div>
            ) : (
              roots.map((root) => (
                <div key={root} className="root-item">
                  <span>{root}</span>
                  <button
                    className="ghost small"
                    onClick={() => handleRemoveRoot(root)}
                  >
                    移除
                  </button>
                </div>
              ))
            )}
          </div>
        </div>
        {scanResult ? (
          <div className="stats">
            <span>扫描文件：{scanResult.stats.total_files}</span>
            <span>命中：{scanResult.stats.matched_files}</span>
            <span>未命中：{scanResult.stats.unmatched_files}</span>
            <span>IOC 物种数：{scanResult.total_species}</span>
          </div>
        ) : (
          <div className="stats">等待扫描</div>
        )}
        {error ? <div className="error">{error}</div> : null}
      </section>

      <main className="main-grid">
        <section className="panel tree">
          <h2>分类树</h2>
          {scanResult ? (
            <>
              <div className="tree-search">
                <input
                  type="search"
                  value={treeQuery}
                  onChange={(event) => setTreeQuery(event.target.value)}
                  placeholder="搜索物种（中文或拉丁名）"
                  aria-label="搜索物种（中文或拉丁名）"
                />
              </div>
              <TreeView
                tree={scanResult.tree}
                query={treeQuery}
                onSelect={setSelectedSpecies}
              />
            </>
          ) : (
            <div className="empty">尚未生成分类树</div>
          )}
        </section>

        <section className="panel gallery">
          <h2>缩略图</h2>
          {selectedSpecies ? (
            <div className="grid">
              {selectedSpecies.photos.map((photo) => (
                <button
                  key={photo.path}
                  className={
                    selectedPhoto?.path === photo.path
                      ? "photo active"
                      : "photo"
                  }
                  onClick={() => setSelectedPhoto(photo)}
                  onDoubleClick={() => handleOpen(photo.path)}
                >
                  {thumbnailErrorMap[photo.path] ? (
                    <div
                      className="photo-fallback"
                      role="img"
                      aria-label={`无法预览：${photo.file_name}`}
                    >
                      无法预览
                    </div>
                  ) : (
                    <img
                      src={toThumbnailSrc(photo.path)}
                      alt={photo.file_name}
                      onError={() => handleThumbnailError(photo.path)}
                    />
                  )}
                  <span>{photo.file_name}</span>
                </button>
              ))}
            </div>
          ) : (
            <div className="empty">请选择一个物种</div>
          )}
        </section>

        <section className="panel meta">
          <h2>元数据</h2>
          {selectedSpecies ? (
            <div className="meta-block">
              <div className="meta-title">
                {selectedSpecies.chinese
                  ? `${selectedSpecies.chinese} ${selectedSpecies.latin}`
                  : selectedSpecies.latin}
              </div>
              <div className="meta-row">数量：{selectedSpecies.count}</div>
              <div className="meta-row">
                {selectedPhoto ? selectedPhoto.path : "选择一张照片查看路径"}
              </div>
              <button
                className="ghost"
                onClick={handleReveal}
                disabled={!selectedPhoto}
              >
                定位到文件夹
              </button>
            </div>
          ) : (
            <div className="empty">尚未选择物种</div>
          )}
        </section>
      </main>
    </div>
  );
}

function TreeView({
  tree,
  query,
  onSelect
}: {
  tree: TaxonTree;
  query: string;
  onSelect: (species: SpeciesNode) => void;
}) {
  const normalizedQuery = query.trim().toLowerCase();
  const hasQuery = normalizedQuery.length > 0;

  const filteredOrders = useMemo(() => {
    if (!normalizedQuery) return tree.orders;

    return tree.orders
      .map((order) => {
        const families = order.families
          .map((family) => {
            const genera = family.genera
              .map((genus) => {
                const species = genus.species.filter((item) => {
                  return (
                    item.chinese.toLowerCase().includes(normalizedQuery) ||
                    item.latin.toLowerCase().includes(normalizedQuery)
                  );
                });

                if (species.length === 0) return null;
                return { ...genus, species };
              })
              .filter((genus): genus is GenusNode => genus !== null);

            if (genera.length === 0) return null;
            return { ...family, genera };
          })
          .filter((family): family is FamilyNode => family !== null);

        if (families.length === 0) return null;
        return { ...order, families };
      })
      .filter((order): order is OrderNode => order !== null);
  }, [tree, normalizedQuery]);

  if (hasQuery && filteredOrders.length === 0) {
    return <div className="empty">未找到匹配物种</div>;
  }

  return (
    <div className="tree-root">
      {filteredOrders.map((order) => (
        <details key={order.name} open>
          <summary>{order.name}</summary>
          {order.families.map((family) => (
            <details key={family.name} className="level" open={hasQuery || undefined}>
              <summary>{family.name}</summary>
              {family.genera.map((genus) => (
                <details key={genus.name} className="level" open={hasQuery || undefined}>
                  <summary>{genus.name}</summary>
                  <div className="species-list">
                    {genus.species.map((species) => (
                      <button
                        key={species.latin}
                        className="species"
                        onClick={() => onSelect(species)}
                      >
                        {species.chinese
                          ? `${species.chinese} ${species.latin}`
                          : species.latin}{" "}
                        ({species.count})
                      </button>
                    ))}
                  </div>
                </details>
              ))}
            </details>
          ))}
        </details>
      ))}
    </div>
  );
}
