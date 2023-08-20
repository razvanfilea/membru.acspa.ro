import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";
import React, {useEffect, useMemo, useState} from "react";
import {Profile, Reservation} from "../../types/wrapper";
import {Card, Center, Grid, Group, Indicator, Loader, Select, Space, Stack, Text} from "@mantine/core";
import {DatePicker} from "@mantine/dates";
import 'dayjs/locale/ro';
import {dateToISOString} from "../../utils/date";
import {useExitIfNotFounder} from "../../utils/admin_tools";
import useProfileData from "../../hooks/useProfileData";

export default function SituationPage() {
    const supabase = useSupabaseClient<Database>()

    const [allProfiles, setAllProfiles] = useState<Profile[]>([])
    const [reservations, setReservations] = useState<Reservation[]>([])
    const [isLoading, setIsLoading] = useState(true)
    const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
    const [dateRange, setDateRange] = useState<[Date | null, Date | null]>([null, null]);
    const [startRange, endRange] = dateRange;

    useExitIfNotFounder();

    useEffect(() => {
        supabase.from('profiles').select('*')
            .order('name', {ascending: true})
            .then(value => {
                if (value.data != null) {
                    setAllProfiles(value.data)
                    setIsLoading(false)
                }
            })
        // We only want to run it once
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    useEffect(() => {
        if (selectedProfileId != null) {
            supabase.from('rezervari').select('*')
                .eq('user_id', selectedProfileId)
                .order('start_date', {ascending: false})
                .order('start_hour', {ascending: true})
                .then(value => {
                    if (value.data != null) {
                        setReservations(value.data)
                    }
                })
        }
    }, [selectedProfileId, supabase])

    const selectedProfile = useMemo(
        () => allProfiles.find(it => it.id === selectedProfileId) || null,
        [allProfiles, selectedProfileId]
    )
    const filteredReservations = useMemo(
        () => {
            if (startRange == null || endRange == null) {
                return []
            }
            return reservations.filter(it => it.user_id === selectedProfileId && new Date(it.start_date) >= startRange && new Date(it.start_date) <= endRange)
        },
        [reservations, selectedProfileId, startRange, endRange]
    )

    return <>
        <Grid
            grow={true}
            columns={4}
        >

            <Grid.Col span={"auto"}>
                <Stack p={'md'}>
                    <Select
                        label="Alege un utilizator"
                        placeholder="Utilizator"
                        searchable
                        transitionProps={{
                            transition: "pop-top-left",
                            duration: 80,
                            timingFunction: "ease"
                        }}
                        data={allProfiles.map(profile => ({value: profile.id, label: profile.name}))}
                        value={selectedProfileId}
                        onChange={setSelectedProfileId}
                    />

                    <DatePicker
                        type='range'
                        value={dateRange}
                        onChange={setDateRange}
                        size={"lg"}
                        locale="ro"
                        renderDay={(date) => {
                            const disabled = !filteredReservations.some(it =>
                                it.start_date === dateToISOString(date) && it.cancelled === false);

                            return (
                                <Indicator
                                    size={8} color="green" offset={-5}
                                    disabled={disabled}>
                                    {date.getDate()}
                                </Indicator>
                            );
                        }}
                    />
                </Stack>
            </Grid.Col>

            <Grid.Col span={2}>
                <Stack p={'md'} id={"report"}
                       sx={(theme) => ({backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[7] : theme.colors.gray[0]})}>
                    {selectedProfile &&
                        SelectedUserReservations(selectedProfile, filteredReservations)
                    }
                </Stack>
            </Grid.Col>
        </Grid>

        <Space h="xl"/>
    </>
}

function SelectedUserReservations(profile: Profile, reservations: Reservation[]) {
    const approvedReservations = reservations.filter((it) => !it.cancelled)
    const cancelledReservations = reservations.filter((it) => it.cancelled)

    return <>
        <Text size={'xl'}>Total rezervări: {approvedReservations.length}</Text>

        {
            approvedReservations.map((reservation, index) => {
                return <Card key={reservation.id}>
                    <Group position={'apart'}>
                        <Text>{index + 1}</Text>

                        <Text weight={900}>{(new Date(reservation.start_date)).toLocaleDateString('ro-RO')}</Text>

                        <Text>De
                            la <b>{reservation.start_hour}:{'00'}</b> la <b>{reservation.start_hour + reservation.duration}:{'00'}</b></Text>
                    </Group>
                </Card>
            })
        }

        <Space h={'lg'}/>

        <Text size={'xl'}>Total rezervări anulate: {cancelledReservations.length}</Text>
        {
            cancelledReservations.map((reservation, index) => {
                return <Card key={reservation.id}>
                    <Group position={'apart'}>
                        <Text>{index + 1}</Text>

                        <Text weight={900}>{(new Date(reservation.start_date)).toLocaleDateString('ro-RO')}</Text>

                        <Text>De
                            la <b>{reservation.start_hour}:{'00'}</b> la <b>{reservation.start_hour + reservation.duration}:{'00'}</b></Text>
                    </Group>
                </Card>
            })
        }
    </>
}
