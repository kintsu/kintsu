import random
from datetime import datetime, timedelta
from typing import List, Dict


def get_downloads_mocked(version_ids: List[int], days: int = 90) -> List[Dict]:
    today = datetime.now().date()
    results = []

    for version_id in version_ids:
        baseline = random.randint(50, 50000)

        for day_offset in range(days):
            day = today - timedelta(days=day_offset)

            variation = random.uniform(0.5, 1.5)
            downloads = int(baseline * variation * (float(day_offset) / days) * 100)

            recency_boost = 1.0 + (1.0 - day_offset / days) * 0.3
            downloads = int(downloads * recency_boost)

            downloads = max(1, downloads)

            results.append(
                {"version": version_id, "day": day.isoformat(), "downloads": downloads}
            )

    return results


def generate_sql_insert(data: List[Dict]) -> str:
    # Build values list
    values = []
    for record in data:
        values.append(
            f"({record['version']}, '{record['day']}', {record['downloads']})"
        )

    values_str = ",\n    ".join(values)

    sql = f"""INSERT INTO downloads (version, day, count)
VALUES
    {values_str};"""

    return sql


if __name__ == "__main__":
    version_ids = [1, 2, 3]

    mock_data = get_downloads_mocked(version_ids, days=365)

    sql = generate_sql_insert(mock_data)
    print(sql)
