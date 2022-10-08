import {GameTable, LocationName} from "./wrapper";

export interface Room {
    readonly locationName: LocationName
    readonly tables: GameTable[]
    readonly maxReservations: number
    readonly startHour: number
    readonly endHour: number
    readonly duration: number
    readonly weekendStartHour: number
    readonly weekendEndHour: number
    readonly weekendDuration: number
}

export class SelectedTable {
    readonly startHour: number
    readonly table: GameTable

    constructor(startHour: number, table: GameTable) {
        this.startHour = startHour
        this.table = table
    }
}
