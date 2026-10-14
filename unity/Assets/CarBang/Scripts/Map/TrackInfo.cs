// Геометрия трассы для ИИ и эвакуации: осевая кольца, срезка через центр, граф проходов.
using System.Collections.Generic;
using UnityEngine;

namespace CarBang
{
    public class TrackInfo
    {
        public static TrackInfo Current;

        public readonly List<Vector3> ring = new List<Vector3>();
        public readonly List<Vector3> ringDir = new List<Vector3>();
        public float a, b, width;

        // граф: узлы кольца + узлы срезки; рёбра двусторонние
        public readonly List<Vector3> nodes = new List<Vector3>();
        public readonly List<List<int>> edges = new List<List<int>>();

        public TrackInfo(float a, float b, float width, int segments)
        {
            this.a = a;
            this.b = b;
            this.width = width;
            for (int i = 0; i < segments; i++)
            {
                float t = i / (float)segments * Mathf.PI * 2f;
                ring.Add(Point(t));
                // касательная против часовой стрелки (если смотреть сверху)
                ringDir.Add(new Vector3(-a * Mathf.Sin(t), 0f, b * Mathf.Cos(t)).normalized);
            }
            for (int i = 0; i < segments; i++)
            {
                nodes.Add(ring[i]);
                edges.Add(new List<int> { (i + 1) % segments, (i + segments - 1) % segments });
            }
        }

        public Vector3 Point(float t) => new Vector3(a * Mathf.Cos(t), 0f, b * Mathf.Sin(t));

        /// <summary>Добавляет срезку: цепочку узлов между двумя точками кольца.</summary>
        public void AddShortcut(int ringA, int ringB, int steps)
        {
            int prev = ringA;
            for (int s = 1; s < steps; s++)
            {
                var p = Vector3.Lerp(ring[ringA], ring[ringB], s / (float)steps);
                nodes.Add(p);
                edges.Add(new List<int>());
                int id = nodes.Count - 1;
                Link(prev, id);
                prev = id;
            }
            Link(prev, ringB);
        }

        void Link(int x, int y)
        {
            edges[x].Add(y);
            edges[y].Add(x);
        }

        public int NearestRing(Vector3 p)
        {
            int best = 0;
            float bd = float.MaxValue;
            for (int i = 0; i < ring.Count; i++)
            {
                float d = (ring[i] - p).sqrMagnitude;
                if (d < bd) { bd = d; best = i; }
            }
            return best;
        }

        public int NearestNode(Vector3 p)
        {
            int best = 0;
            float bd = float.MaxValue;
            for (int i = 0; i < nodes.Count; i++)
            {
                float d = (nodes[i] - p).sqrMagnitude;
                if (d < bd) { bd = d; best = i; }
            }
            return best;
        }

        /// <summary>Кратчайший путь по графу (Дейкстра: узлов около сотни).</summary>
        public List<int> Path(int from, int to)
        {
            int n = nodes.Count;
            var dist = new float[n];
            var prev = new int[n];
            var done = new bool[n];
            for (int i = 0; i < n; i++) { dist[i] = float.MaxValue; prev[i] = -1; }
            dist[from] = 0f;
            for (int it = 0; it < n; it++)
            {
                int u = -1;
                float best = float.MaxValue;
                for (int i = 0; i < n; i++) if (!done[i] && dist[i] < best) { best = dist[i]; u = i; }
                if (u < 0 || u == to) break;
                done[u] = true;
                foreach (var v in edges[u])
                {
                    float nd = dist[u] + Vector3.Distance(nodes[u], nodes[v]);
                    if (nd < dist[v]) { dist[v] = nd; prev[v] = u; }
                }
            }
            var path = new List<int>();
            for (int v = to; v >= 0; v = prev[v])
            {
                path.Add(v);
                if (v == from) break;
            }
            path.Reverse();
            return path;
        }

        /// <summary>Точка и направление для эвакуации машины: ближайшая точка кольца.</summary>
        public void ResetPose(Vector3 near, Vector3 currentFwd, out Vector3 pos, out Quaternion rot)
        {
            int i = NearestRing(near);
            var dir = ringDir[i];
            if (Vector3.Dot(dir, currentFwd) < 0f) dir = -dir;
            pos = ring[i] + Vector3.up * 1.2f;
            rot = Quaternion.LookRotation(dir, Vector3.up);
        }
    }
}
