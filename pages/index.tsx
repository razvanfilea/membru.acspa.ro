import React, {useEffect, useMemo, useState} from "react";
import Head from "next/head";
import {Button, Card, Divider, Grid, Group, Paper, Radio, Space, Stack, Text, Title} from "@mantine/core";
import {Calendar} from '@mantine/dates';
import {BaseReservation, getEndDateDuration, Reservation} from "../model/Reservation";
import GameTable from "../model/GameTable";
import {useScrollIntoView} from "@mantine/hooks";
import {appwrite, userIsLoggedIn} from "../utils/appwrite_utils";
import {NextLink} from "@mantine/next";
import ReservationComponent from "../components/Reservation";
import {LocationName} from "../model/Room";
import 'dayjs/locale/ro'

function addDays(date, days) {
    const result = new Date(date);
    result.setDate(result.getDate() + days);
    return result;
}

interface IParams {
    gara: Room,
    boromir: Room,
    daysAhead: number
}

interface Room {
    tables: GameTable[]
    startHour: number
    endHour: number
    duration: number
}

class SelectedTable {
    readonly startHour: number
    readonly table: GameTable

    constructor(startHour: number, table: GameTable) {
        this.startHour = startHour
        this.table = table
    }
}

export default function MakeReservationPage(params: IParams): JSX.Element {
    const minRange = addDays(new Date, 1)
    const maxRange = addDays(minRange, params.daysAhead)

    const [locationName, setLocationName] = useState(LocationName.Gara)
    const [selectedDate, setSelectedDate] = useState<Date>(null)
    const [selectedTable, setSelectedTable] = useState<SelectedTable>(null)
    const [isLoggedIn, setIsLoggedIn] = useState(false)

    useEffect(() => {
        userIsLoggedIn().then((value) => setIsLoggedIn(value))
    }, [])

    function onSelectedDateChange(selectedDate: Date) {
        setSelectedDate(selectedDate)
        setSelectedTable(null)
    }

    const room = locationName == LocationName.Gara ? params.gara : params.boromir;

    return (<>
        <Head>
            <title>Rezervări</title>
            <meta name="viewport" content="initial-scale=1.0, width=device-width"/>
        </Head>

        <Title>Rezervări</Title>

        <Space h="lg"/>

        <Paper>
            {!isLoggedIn &&
                <>
                    <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                        backgroundColor: theme.colors.orange,
                        marginTop: theme.spacing.sm
                    })}>
                        <Text>Trebuie să ai un cont și să fi un membru pentru a putea face rezervări!</Text>
                    </Paper>
                    <Space h="md"/>
                </>
            }

            <Grid columns={12} gutter={"xl"}>
                <Grid.Col md={6} lg={6} xl={4} key={"calendar"}>
                    <Stack>
                        <Radio.Group
                            value={locationName}
                            onChange={(value) => {
                                switch (value) {
                                    case LocationName.Gara.toString():
                                        setLocationName(LocationName.Gara);
                                        break;
                                    case LocationName.Boromir.toString():
                                        setLocationName(LocationName.Boromir);
                                        break;
                                }
                            }}
                            label={"Alege locația unde faci rezervarea:"}
                            size="md">
                            <Radio value={LocationName.Gara} label={"Gară"}/>
                            <Radio value={LocationName.Boromir} label={"Boromir"}/>
                        </Radio.Group>

                        <Space h={"sm"} />

                        <Text>Alege ziua rezervării:</Text>

                        <Calendar
                            minDate={minRange}
                            maxDate={maxRange}
                            hideOutsideDates={true}
                            size={"xl"}
                            locale={"ro"}
                            value={selectedDate}
                            onChange={(date) => {
                                if (isLoggedIn && date != null) onSelectedDateChange(date)
                            }}
                            excludeDate={(date) => date.getDay() === 0}
                            fullWidth={true}
                        />
                    </Stack>
                </Grid.Col>

                <Grid.Col md={6} lg={6} xl={3} key={"tables"}>
                    {SelectGameTable(room, selectedDate, selectedTable, setSelectedTable)}
                </Grid.Col>

                <Grid.Col md={12} lg={12} xl={5} key={"confirm"}>
                    {ConfirmSelection(locationName, room.duration, selectedDate, selectedTable)}
                </Grid.Col>
            </Grid>
        </Paper>
    </>);
}

function SelectGameTable(room: Room,
                         selectedDate: Date,
                         selectedTable: SelectedTable,
                         onSelectTable: (s: SelectedTable) => void): JSX.Element {

    const [reservations, setReservations] = useState<Reservation[]>([])

    useEffect(() => {
        function refetchReservations() {
            appwrite.database.listDocuments<Reservation>('62cdcab0e527f917eb34').then((res) => {
                setReservations(res.documents)
                console.log("Reservation updated")
            });
        }

        refetchReservations();

        return appwrite.client.subscribe('collections.62cdcab0e527f917eb34.documents', response => {
            refetchReservations()
            console.log(response);
        });
    }, [])

    const selectedDayReservations = useMemo(() => {
        if (selectedDate) {
            return reservations.filter((reservation) => {
                return (new Date(reservation.date)).getDay() == selectedDate.getDay()
            })
        } else {
            return []
        }
    }, [reservations, selectedDate])

    if (selectedDate == null) return null

    const selectedTableId = selectedTable?.table.$id;
    const selectedStartHour = (selectedTable) ? selectedTable.startHour : -1;

    const tableButtons = function (startHour: number) {
        return room.tables.map((gameTable) => {
            return (
                <Button
                    variant={(gameTable.$id == selectedTableId && startHour == selectedStartHour) ? "filled" : "outline"}
                    key={gameTable.$id}
                    disabled={selectedDayReservations.some((res) => gameTable.$id == res.table_id && startHour == res.start_hour)}
                    onClick={() => onSelectTable(new SelectedTable(startHour, gameTable))}>
                    {gameTable.name}
                </Button>
            )
        })
    }

    const hourButtons = (startHour) => {
        return (<Stack key={startHour}>
            <Text>{`Ora ${startHour} - ${startHour + room.duration}`}</Text>
            <Group position={"apart"} style={{marginLeft: "1em", marginRight: "1em"}}>
                {tableButtons(startHour)}
            </Group>
            <Divider variant={"dashed"}/>
        </Stack>)
    };

    const allButtons = () => {
        let content = [];
        for (let i = room.startHour; i < room.endHour; i += room.duration) {
            content.push(hourButtons(i));
        }
        return content;
    };

    return (<Stack>
        <Text weight={600}>Data selectată: <Text color={"blue"}>{selectedDate.toLocaleDateString('ro-RO')}</Text></Text>

        {allButtons()}
    </Stack>)
}

function ConfirmSelection(locationName: LocationName, duration: number, selectedDate: Date, selectedTable: SelectedTable): JSX.Element {
    enum ConfirmationStatus {
        None,
        Loading,
        Success,
        Fail
    }

    const [status, setStatus] = useState(ConfirmationStatus.None)
    const {scrollIntoView, targetRef} = useScrollIntoView<HTMLDivElement>({});

    useEffect(() => {
        if (selectedDate && selectedTable) {
            scrollIntoView({alignment: 'center'})
            setStatus(ConfirmationStatus.None)
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [selectedDate, selectedTable])

    if (selectedDate == null || selectedTable == null) return null

    const startDate = getEndDateDuration(selectedDate, selectedTable.startHour * 60)

    const baseReservation: BaseReservation = {
        start_date: startDate.toISOString(),
        duration: duration,
        table_id: selectedTable.table.$id,
        user_id: "",
        location: locationName
    }

    const gameTable = selectedTable.table;

    return (<Card p={"xl"} shadow={"sm"}>
        <div style={{marginTop: 'sm'}}>
            <Title ref={targetRef}>Confirmă rezervarea:</Title>
        </div>

        <Space h={"lg"}/>

        {ReservationComponent(baseReservation, gameTable, false)}

        <Space h={"md"}/>

        {(status == ConfirmationStatus.None || status == ConfirmationStatus.Loading) &&
            <Button fullWidth style={{marginTop: 14}}
                    loading={status == ConfirmationStatus.Loading}
                    onClick={async () => {
                        setStatus(ConfirmationStatus.Loading)
                        const success = await publishReservation(baseReservation)
                        setStatus(success ? ConfirmationStatus.Success : ConfirmationStatus.Fail)
                    }}>Confirmă rezervarea!</Button>
        }

        {status == ConfirmationStatus.Success &&
            <Stack>
                <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                    backgroundColor: theme.colors.green,
                    marginTop: theme.spacing.sm,
                    marginBottom: theme.spacing.xs
                })}>
                    <Text align={"center"} color="#FFF"><b>Cererea ta de rezervare a fost înregistrată</b></Text>
                </Paper>

                <Group align={"center"}>
                    <Text weight={600}>Verifică pe pagina ta de profil dacă rezervarea a fost aprobată:</Text>
                    <NextLink href={"/profile"}>
                        <Button variant={'light'}>Vezi profilul</Button>
                    </NextLink>
                </Group>
            </Stack>
        }

        {status == ConfirmationStatus.Fail &&
            <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                backgroundColor: theme.colors.orange,
                marginTop: theme.spacing.sm,
                marginBottom: theme.spacing.xs
            })}>
                <Text align={"center"} color="#FFF">Rezervarea nu a putut fi realizată.</Text>
            </Paper>
        }
    </Card>)
}

async function publishReservation(baseReservation: BaseReservation): Promise<boolean> {
    const reservation = {
        ...baseReservation,
        user_id: (await appwrite.account.getSession('current')).userId
    }

    console.log(reservation)

    try {
        await appwrite.database.createDocument<Reservation>("62cdcab0e527f917eb34", "unique()", reservation, ["role:member"])
        return true
    } catch (e) {
        console.log(e)
        return false
    }
}

export async function getStaticProps({}) {
    const gameTables = await appwrite.database.listDocuments<GameTable>("62cdcac1bb2c8a4e5e48")
    const garaTables = gameTables.documents.filter((value) => value.location == LocationName.Gara)
    const boromirTables = gameTables.documents.filter((value) => value.location == LocationName.Boromir)

    const props: IParams = {
        daysAhead: 14,
        gara: {
            tables: garaTables,
            startHour: 10,
            endHour: 21,
            duration: 1,
        },
        boromir: {
            tables: boromirTables,
            startHour: 18,
            endHour: 22,
            duration: 2,
        }
    }

    return {
        props: props
    }
}
