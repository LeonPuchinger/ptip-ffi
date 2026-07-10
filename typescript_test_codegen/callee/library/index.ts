export function trim_whitespace(str: string): string {
    return str.trim();
}

export class Point {
    x: number;
    y: number;

    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }

    distance_to_origin(): number {
        return Math.sqrt(this.x * this.x + this.y * this.y);
    }
}

export function takes_point(point: Point): number {
    return point.distance_to_origin();
}
