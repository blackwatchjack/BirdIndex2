import { useEffect, useMemo, useState } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
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

export default function App() {
  const [iocPath, setIocPath] = useState("Multiling IOC 15.1_d.xlsx");
  const [cachePath, setCachePath] = useState("");
  const [roots, setRoots] = useState<string[]>([]);
  const [scanResult, setScanResult] = useState<ScanResponse | null>(null);
  const [selectedSpecies, setSelectedSpecies] = useState<SpeciesNode | null>(null);
  const [selectedPhoto, setSelectedPhoto] = useState<PhotoItem | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    appDataDir()
      .then((dir) => join(dir, "birdindex2", "cache.json"))
      .then((path) => setCachePath(path))
      .catch(() => setCachePath(""));
  }, []);

  const rootsLabel = useMemo(() => {
    if (roots.length === 0) return "0 个目录";
    return `${roots.length} 个目录`;
  }, [roots.length]);

  const handlePickRoots = async () => {
    const selected = await open({
      directory: true,
      multiple: true,
      title: "选择根目录"
    });
    if (!selected) return;
    const next = Array.isArray(selected) ? selected : [selected];
    setRoots((prev) => Array.from(new Set([...prev, ...next])));
  };

  const handleRemoveRoot = (path: string) => {
    setRoots((prev) => prev.filter((item) => item !== path));
  };

  const handleClearRoots = () => {
    setRoots([]);
  };

  const handleScan = async () => {
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
        <label>
          缓存路径
          <input
            value={cachePath}
            onChange={(event) => setCachePath(event.target.value)}
            placeholder="cache.json"
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
            <TreeView tree={scanResult.tree} onSelect={setSelectedSpecies} />
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
                  <img src={convertFileSrc(photo.path)} alt={photo.file_name} />
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
  onSelect
}: {
  tree: TaxonTree;
  onSelect: (species: SpeciesNode) => void;
}) {
  return (
    <div className="tree-root">
      {tree.orders.map((order) => (
        <details key={order.name} open>
          <summary>{order.name}</summary>
          {order.families.map((family) => (
            <details key={family.name} className="level">
              <summary>{family.name}</summary>
              {family.genera.map((genus) => (
                <details key={genus.name} className="level">
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
